use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use git2::Repository;
use tracing::debug;

use crate::shared::Repo;

/// Marks a reusable workflow reference as living in this repository.
pub const LOCAL_PREFIX: &str = "./";

/// Rewrites local reusable workflow references into permalinks into the source repository.
pub struct Rewriter {
    source: Repo,
    root: PathBuf,
    /// Commit every rewritten reference is pinned to, shared by all files in a run.
    sha: String,
}

impl Rewriter {
    /// Build a rewriter for workflows checked out under `root`, pinning to `sha`.
    pub fn new(source: Repo, root: PathBuf, sha: String) -> Self {
        Self { source, root, sha }
    }

    /// Replace every `uses: ./path` with `owner/name/path@sha`.
    ///
    /// A reference that does not resolve is an error rather than a passthrough: the
    /// unrewritten `./` form is valid YAML but resolves against the *target* repository,
    /// so shipping it would silently point every target at a workflow it does not have.
    pub fn rewrite(&self, contents: &str) -> Result<String> {
        let mut rewritten = Vec::new();
        for line in contents.lines() {
            match parse_local_reference(line) {
                Some(reference) => rewritten.push(self.rewrite_reference(reference)?),
                None => rewritten.push(line.to_string()),
            }
        }

        let mut rewritten = rewritten.join("\n");
        // `lines` drops the trailing newline; keep the source file's own ending.
        if contents.ends_with('\n') {
            rewritten.push('\n');
        }

        Ok(rewritten)
    }

    // Rebuild one reference line with the original indentation and the run's commit.
    fn rewrite_reference(&self, reference: LocalReference<'_>) -> Result<String> {
        // Guard against a typo becoming a broken `uses:` in every target repository.
        // Either form is valid: a reusable workflow file, or an action directory.
        if !self.root.join(reference.path).exists() {
            bail!(
                "local workflow reference '{}{}' does not exist under {}",
                LOCAL_PREFIX,
                reference.path,
                self.root.display()
            );
        }

        debug!(path = reference.path, sha = %self.sha, "rewriting local reference");

        Ok(format!(
            "{}uses: {}/{}/{}@{}",
            reference.prefix, self.source.owner, self.source.repository, reference.path, self.sha
        ))
    }
}

// A `uses: ./path` reference split into the parts needed to rebuild the line.
struct LocalReference<'a> {
    /// Everything before `uses:`, preserving indentation and any sequence dash.
    prefix: &'a str,
    path: &'a str,
}

// Recognize `uses: ./path`, ignoring any trailing comment the source line carried.
fn parse_local_reference(line: &str) -> Option<LocalReference<'_>> {
    // `uses:` appears both as a mapping key and as the first key of a sequence item.
    let key = line.find("uses:")?;
    let (prefix, rest) = line.split_at(key);
    if !prefix
        .chars()
        .all(|character| character.is_whitespace() || character == '-')
    {
        return None;
    }

    let value = rest.strip_prefix("uses:")?.trim_start();
    let path = value.strip_prefix(LOCAL_PREFIX)?;

    // A reference may be followed by a comment; the path itself cannot contain whitespace.
    let path = path.split_whitespace().next()?;
    // An action directory is commonly written with a trailing slash, which a
    // `owner/name/path@sha` reference cannot carry.
    let path = path.trim_end_matches('/');
    (!path.is_empty()).then_some(LocalReference { prefix, path })
}

/// Return the commit currently checked out at `root`.
///
/// Sync is dispatched from `main` only, so this commit is always reachable from it.
pub fn head_sha(root: &Path) -> Result<String> {
    let repository = Repository::discover(root)
        .with_context(|| format!("discovering git repository at {}", root.display()))?;

    let head = repository
        .head()
        .context("reading HEAD")?
        .peel_to_commit()
        .context("resolving HEAD to a commit")?;

    Ok(head.id().to_string())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, time::SystemTime};

    use non_empty_string::NonEmptyString;

    use super::{Rewriter, parse_local_reference};
    use crate::shared::Repo;

    fn source() -> Repo {
        Repo::new(
            NonEmptyString::new("mongodb".to_string()).expect("non-empty owner"),
            NonEmptyString::new("apix-action".to_string()).expect("non-empty name"),
        )
    }

    // Create a checkout root containing `_close-jira.yaml`, the workflow tests reference.
    fn root_with_workflow(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("apix-refs-{label}-{}-{unique}", std::process::id()));
        let directory = root.join(".github/workflows");
        fs::create_dir_all(&directory).expect("create workflow directory");
        fs::write(directory.join("_close-jira.yaml"), "name: reusable\n").expect("write workflow");
        root
    }

    fn rewriter(root: PathBuf) -> Rewriter {
        Rewriter::new(source(), root, "abc123".to_string())
    }

    #[test]
    fn rewrite_replaces_local_reference_with_permalink() {
        let root = root_with_workflow("permalink");

        let contents = "jobs:\n  close-jira:\n    uses: ./.github/workflows/_close-jira.yaml\n";

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            "jobs:\n  close-jira:\n    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@abc123\n"
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_leaves_remote_references_untouched() {
        let root = root_with_workflow("remote");

        let contents = "    uses: actions/checkout@v7\n    with:\n      path: ./local\n";

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(rewritten, contents);

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_drops_trailing_comment_from_source_line() {
        let root = root_with_workflow("comment");

        let contents = "    uses: ./.github/workflows/_close-jira.yaml # reusable\n";

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            "    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@abc123\n"
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_pins_every_reference_to_the_same_commit() {
        let root = root_with_workflow("shared");
        fs::create_dir_all(root.join("create-jira")).expect("create action directory");

        let contents = concat!(
            "    uses: ./.github/workflows/_close-jira.yaml\n",
            "      - uses: ./create-jira/\n"
        );

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            concat!(
                "    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@abc123\n",
                "      - uses: mongodb/apix-action/create-jira@abc123\n"
            )
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_fails_on_missing_workflow() {
        let root = root_with_workflow("missing");

        let contents = "    uses: ./.github/workflows/_typo.yaml\n";

        let error = rewriter(root.clone())
            .rewrite(contents)
            .expect_err("missing file is fatal");

        assert!(error.to_string().contains("_typo.yaml"));

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn parse_local_reference_requires_uses_key() {
        assert!(parse_local_reference("    path: ./.github/workflows/x.yaml").is_none());
        assert!(parse_local_reference("    uses: ./").is_none());
        assert!(parse_local_reference("    uses: .///").is_none());
        assert!(parse_local_reference("    uses: owner/name/x.yaml@sha").is_none());
        // A `uses:` appearing inside another value is not a reference.
        assert!(parse_local_reference("    body: see uses: ./x.yaml").is_none());
        assert!(parse_local_reference("      - uses: ./x.yaml").is_some());
        assert!(parse_local_reference("    uses: ./x.yaml").is_some());
    }
}
