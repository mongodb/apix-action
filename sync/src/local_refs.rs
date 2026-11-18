use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use git2::{Commit, Oid, Repository};

use crate::shared::Repo;

/// Marks a reusable workflow reference as living in this repository.
pub const LOCAL_PREFIX: &str = "./";

/// GitHub's self-repository syntax, which also refers to this repository.
pub const SELF_PREFIX: &str = "$/";

/// Prefixes that mark a `uses:` reference as living in this repository.
const LOCAL_PREFIXES: [&str; 2] = [LOCAL_PREFIX, SELF_PREFIX];

/// The release every rewritten reference is pinned to.
#[derive(Debug)]
pub struct ReleaseRef {
    /// Commit the release tag points at.
    pub sha: String,
    /// Tag name, e.g. `v18`, written as a trailing comment so dependabot can map a
    /// pinned commit back to the release it came from.
    pub tag: String,
}

/// Rewrites local reusable workflow references into permalinks into the source repository.
pub struct Rewriter {
    source: Repo,
    root: PathBuf,
    /// Release every rewritten reference is pinned to, shared by all files in a run.
    release: ReleaseRef,
}

impl Rewriter {
    /// Build a rewriter for workflows checked out under `root`, pinning to `release`.
    pub fn new(source: Repo, root: PathBuf, release: ReleaseRef) -> Self {
        Self {
            source,
            root,
            release,
        }
    }

    /// Replace every `uses: ./path` or `uses: $/path` with `owner/name/path@sha # tag`.
    ///
    /// A reference that does not resolve is an error rather than a passthrough: the
    /// unrewritten local form is valid YAML but resolves against the *target* repository,
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

    // Rebuild one reference line with the original indentation and the run's release.
    fn rewrite_reference(&self, reference: LocalReference<'_>) -> Result<String> {
        // A reference the release does not have would become a broken `uses:` in every
        // target repository, so fail rather than ship it.
        if !self.exists_at_release(reference.path)? {
            bail!(
                "local workflow reference '{}' does not exist at release {}; tag a release before syncing it",
                reference.path,
                self.release.tag
            );
        }

        Ok(format!(
            "{}uses: {}/{}/{}@{} # {}",
            reference.prefix,
            self.source.owner,
            self.source.repository,
            reference.path,
            self.release.sha,
            self.release.tag
        ))
    }

    /// Returns whether `path` is present at the release commit the run pins to.
    fn exists_at_release(&self, path: &str) -> Result<bool> {
        let repository = self.repository()?;
        let commit = self.release_commit(&repository)?;

        Ok(commit
            .tree()
            .context("reading the release tree")?
            .get_path(Path::new(path))
            .is_ok())
    }

    /// The release every reference is pinned to.
    pub(crate) fn release(&self) -> &ReleaseRef {
        &self.release
    }

    // Open the git repository the checkout lives in.
    pub(crate) fn repository(&self) -> Result<Repository> {
        Repository::discover(&self.root)
            .with_context(|| format!("discovering git repository at {}", self.root.display()))
    }

    // The commit the release tag points at.
    pub(crate) fn release_commit<'repository>(
        &self,
        repository: &'repository Repository,
    ) -> Result<Commit<'repository>> {
        repository
            .find_commit(Oid::from_str(&self.release.sha).context("parsing the release commit")?)
            .context("reading the release commit")
    }
}

// A `uses: ./path` or `uses: $/path` reference split into the parts needed to
// rebuild the line.
struct LocalReference<'a> {
    /// Everything before `uses:`, preserving indentation and any sequence dash.
    prefix: &'a str,
    path: &'a str,
}

// Recognize `uses: ./path` or `uses: $/path`, ignoring any trailing comment the
// source line carried.
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
    let local_prefix = LOCAL_PREFIXES
        .iter()
        .find(|candidate| value.starts_with(**candidate))?;
    let path = value.strip_prefix(*local_prefix)?;

    // A reference may be followed by a comment; the path itself cannot contain whitespace.
    let path = path.split_whitespace().next()?;
    // An action directory is commonly written with a trailing slash, which a
    // `owner/name/path@sha` reference cannot carry.
    let path = path.trim_end_matches('/');
    (!path.is_empty()).then_some(LocalReference { prefix, path })
}

/// Resolve the highest `v<number>` tag at `root` to the commit it points at.
pub fn release_ref(root: &Path) -> Result<ReleaseRef> {
    let repository = Repository::discover(root)
        .with_context(|| format!("discovering git repository at {}", root.display()))?;

    let tags = repository.tag_names(None).context("listing tags")?;
    // A name that is not valid UTF-8 cannot be a `v<number>` tag, so unreadable entries
    // are skipped rather than treated as an error.
    let names = tags.iter().filter_map(|name| name.ok().flatten());
    let tag = highest_release(names).with_context(|| {
        format!(
            "no v<number> release tag found under {}; sync pins to a release",
            root.display()
        )
    })?;

    let sha = repository
        .find_reference(&format!("refs/tags/{tag}"))
        .with_context(|| format!("reading tag {tag}"))?
        .peel_to_commit()
        .with_context(|| format!("resolving tag {tag} to a commit"))?
        .id()
        .to_string();

    Ok(ReleaseRef {
        sha,
        tag: tag.to_string(),
    })
}

/// Pick the highest `v<number>` tag, ignoring anything that is not one.
///
/// Compared numerically, so `v18` outranks `v9`.
fn highest_release<'a>(names: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    names
        .filter_map(|name| {
            let number = name.strip_prefix('v')?.parse::<u64>().ok()?;
            Some((number, name))
        })
        .max_by_key(|(number, _)| *number)
        .map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::SystemTime,
    };

    use git2::{Oid, Repository, Signature};
    use non_empty_string::NonEmptyString;

    use super::{Rewriter, highest_release, parse_local_reference, release_ref};
    use crate::shared::Repo;

    fn source() -> Repo {
        Repo::new(
            NonEmptyString::new("mongodb".to_string()).expect("non-empty owner"),
            NonEmptyString::new("apix-action".to_string()).expect("non-empty name"),
        )
    }

    /// Create a checkout that is a git repository holding `_close-jira.yaml` and
    /// `create-jira/action.yaml` — the paths the tests reference — with its single commit
    /// tagged `v18`.
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
        // An action directory is referenced by path, and git does not track empty ones.
        fs::create_dir_all(root.join("create-jira")).expect("create action directory");
        fs::write(root.join("create-jira/action.yaml"), "name: action\n").expect("write action");
        commit_and_tag(&root, "v18");
        root
    }

    /// Commit everything under `root`, returning the new commit.
    fn commit_all(root: &Path, message: &str) -> Oid {
        let repository = Repository::open(root).expect("open repository");
        let mut index = repository.index().expect("open index");
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .expect("stage files");
        index.write().expect("write index");
        let tree = repository
            .find_tree(index.write_tree().expect("write tree"))
            .expect("read tree");
        let signature = Signature::now("test", "test@example.com").expect("signature");
        let parent = repository
            .head()
            .ok()
            .map(|head| head.peel_to_commit().expect("peel"));
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parents,
            )
            .expect("commit")
    }

    /// Initialize `root` as a repository with its files committed and a lightweight `tag`
    /// on that commit.
    fn commit_and_tag(root: &Path, tag: &str) {
        Repository::init(root).expect("init repository");
        let head = commit_all(root, "init");
        // A lightweight tag is a ref pointing straight at the commit, which is what the
        // repository's own releases use.
        Repository::open(root)
            .expect("open repository")
            .reference(&format!("refs/tags/{tag}"), head, false, "add tag")
            .expect("tag");
    }

    /// The commit the fixture's tag points at, resolved without going through
    /// `release_ref` so the two are checked against each other.
    fn tag_sha(root: &Path, tag: &str) -> String {
        Repository::open(root)
            .expect("open repository")
            .find_reference(&format!("refs/tags/{tag}"))
            .expect("find tag")
            .peel_to_commit()
            .expect("peel")
            .id()
            .to_string()
    }

    /// The commit `root` is checked out at.
    fn head_sha(root: &Path) -> String {
        Repository::open(root)
            .expect("open repository")
            .head()
            .expect("head")
            .peel_to_commit()
            .expect("peel")
            .id()
            .to_string()
    }

    fn rewriter(root: PathBuf) -> Rewriter {
        let release = release_ref(&root).expect("resolve release");
        Rewriter::new(source(), root, release)
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
            format!(
                "jobs:\n  close-jira:\n    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@{} # v18\n",
                tag_sha(&root, "v18")
            )
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_replaces_self_repository_reference_with_permalink() {
        let root = root_with_workflow("self-repo");

        let contents = concat!(
            "jobs:\n",
            "  close-jira:\n",
            "    uses: $/.github/workflows/_close-jira.yaml\n",
            "  create:\n",
            "    uses: $/create-jira\n"
        );

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            format!(
                concat!(
                    "jobs:\n",
                    "  close-jira:\n",
                    "    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@{} # v18\n",
                    "  create:\n",
                    "    uses: mongodb/apix-action/create-jira@{} # v18\n"
                ),
                tag_sha(&root, "v18"),
                tag_sha(&root, "v18")
            )
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
    fn rewrite_replaces_source_comment_with_the_release() {
        let root = root_with_workflow("comment");

        let contents = "    uses: ./.github/workflows/_close-jira.yaml # reusable\n";

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            format!(
                "    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@{} # v18\n",
                tag_sha(&root, "v18")
            )
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_pins_every_reference_to_the_same_release() {
        let root = root_with_workflow("shared");

        let contents = concat!(
            "    uses: ./.github/workflows/_close-jira.yaml\n",
            "      - uses: ./create-jira/\n"
        );

        let rewritten = rewriter(root.clone())
            .rewrite(contents)
            .expect("rewrite succeeds");

        assert_eq!(
            rewritten,
            format!(
                concat!(
                    "    uses: mongodb/apix-action/.github/workflows/_close-jira.yaml@{} # v18\n",
                    "      - uses: mongodb/apix-action/create-jira@{} # v18\n"
                ),
                tag_sha(&root, "v18"),
                tag_sha(&root, "v18")
            )
        );

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn rewrite_fails_when_the_reference_postdates_the_release() {
        let root = root_with_workflow("post-release");
        // Written after the tag, so absent from the release.
        fs::write(
            root.join(".github/workflows/_new-thing.yaml"),
            "name: new\n",
        )
        .expect("write workflow");

        let contents = "    uses: ./.github/workflows/_new-thing.yaml\n";

        let error = rewriter(root.clone())
            .rewrite(contents)
            .expect_err("a reference the release does not have is fatal");

        assert!(error.to_string().contains("does not exist at release v18"));

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn release_ref_resolves_a_lightweight_tag() {
        let root = root_with_workflow("lightweight");

        let release = release_ref(&root).expect("resolve release");

        assert_eq!(release.tag, "v18");
        assert_eq!(release.sha, tag_sha(&root, "v18"));

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn release_ref_resolves_an_annotated_tag() {
        let root = root_with_workflow("annotated");
        let repository = Repository::open(&root).expect("open repository");
        repository
            .tag_delete("v18")
            .expect("delete lightweight tag");
        let tagger = Signature::now("test", "test@example.com").expect("signature");
        {
            let head = repository
                .head()
                .expect("head")
                .peel_to_commit()
                .expect("peel");
            // `tag` writes a tag object and points the ref at it, unlike the lightweight
            // ref the fixture normally uses.
            repository
                .tag("v18", head.as_object(), &tagger, "release", true)
                .expect("create annotated tag");
        }

        let release = release_ref(&root).expect("resolve release");

        assert_eq!(release.tag, "v18");
        assert_eq!(release.sha, tag_sha(&root, "v18"));

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn release_ref_uses_the_tag_not_the_checkout() {
        let root = root_with_workflow("moved-head");
        // A commit after the tag: HEAD moves on, the release does not.
        fs::write(
            root.join(".github/workflows/_close-jira.yaml"),
            "name: moved\n",
        )
        .expect("write workflow");
        commit_all(&root, "later");

        let release = release_ref(&root).expect("resolve release");

        assert_eq!(release.sha, tag_sha(&root, "v18"));
        assert_ne!(release.sha, head_sha(&root));

        fs::remove_dir_all(root).expect("clean up");
    }

    #[test]
    fn release_ref_errors_without_a_release_tag() {
        let root = root_with_workflow("no-tags");
        Repository::open(&root)
            .expect("open repository")
            .tag_delete("v18")
            .expect("delete tag");

        let error = release_ref(&root).expect_err("no release tag is fatal");

        assert!(error.to_string().contains("no v<number> release tag"));

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
    fn highest_release_picks_the_largest_version() {
        assert_eq!(
            highest_release(["v9", "v18", "v10"].into_iter()),
            Some("v18")
        );
        assert_eq!(highest_release(["main", "v3"].into_iter()), Some("v3"));
        // Not a release tag: no `v` prefix, an empty suffix, or a non-number.
        assert_eq!(
            highest_release(["v", "vNext", "release-3"].into_iter()),
            None
        );
        assert_eq!(highest_release(std::iter::empty()), None);
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
        // GitHub's self-repository syntax is recognized the same way.
        assert!(parse_local_reference("    uses: $/x.yaml").is_some());
        assert!(parse_local_reference("      - uses: $/x.yaml").is_some());
        assert!(parse_local_reference("    uses: $/").is_none());
        assert!(parse_local_reference("    uses: owner/name/x.yaml@sha").is_none());
    }
}
