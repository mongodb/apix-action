use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use git2::{
    Cred, FetchOptions, RemoteCallbacks, Repository, ResetType, StatusOptions,
    build::{CheckoutBuilder, RepoBuilder},
};
use redacted::FullyRedacted;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use crate::{
    scan::MARKER_PREFIX,
    shared::{GithubToken, Repo, SyncWorkflow, expected_workflows},
};

/// Check out all target repositories and write their applicable workflows.
pub async fn sync(
    workflows: &[SyncWorkflow],
    repositories: &[Repo],
    targets: PathBuf,
    token: FullyRedacted<GithubToken>,
) -> Result<()> {
    info!(
        repositories = repositories.len(),
        "checking out repositories"
    );
    let token = Arc::new(token);
    let mut tasks = JoinSet::new();
    // Repository checkout uses blocking git2 calls, so run each checkout off the async runtime.
    for repository in repositories.iter().cloned() {
        let targets = targets.clone();
        let token = Arc::clone(&token);
        tasks.spawn_blocking(move || checkout_repository(repository, &targets, &token));
    }

    let mut first_error = None;
    while let Some(result) = tasks.join_next().await {
        if let Err(task_error) = result
            .context("repository checkout task failed")
            .and_then(|result| result)
        {
            error!(error = %task_error, "repository checkout failed");
            first_error.get_or_insert(task_error);
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }

    // Write each source workflow only to repositories that declared it as a sync target.
    for workflow in workflows {
        for repository in &workflow.sync {
            let workflows_directory = targets
                .join(repository.owner.as_str())
                .join(repository.repository.as_str())
                .join(".github/workflows");
            let target = workflows_directory.join(&workflow.file_name);
            debug!(path = %target.display(), "writing workflow");
            tokio::fs::create_dir_all(&workflows_directory)
                .await
                .context("creating target workflow directory")?;
            tokio::fs::write(&target, &workflow.contents)
                .await
                .context("writing target workflow")?;
            info!(path = %target.display(), "workflow written");
        }
    }

    // Drop previously synced workflows the source no longer sends here.
    remove_orphans(workflows, repositories, &targets).await?;

    Ok(())
}

/// Collect every repository targeted by a workflow sync.
pub fn target_repositories(workflows: &[SyncWorkflow]) -> HashSet<Repo> {
    workflows
        .iter()
        .flat_map(|workflow| workflow.sync.iter().cloned())
        .collect()
}

/// Remove synced workflows a target repository is no longer a sync target for.
///
/// Covers both a workflow deleted upstream and a `# sync ->` header dropped for one
/// repository. Only files carrying the generated marker are eligible, so workflows a
/// target owns itself are never touched.
///
/// Repositories are supplied by the caller, so a repository dropped from every header is
/// still swept as long as it appears in the app installation.
pub async fn remove_orphans(
    workflows: &[SyncWorkflow],
    repositories: &[Repo],
    targets: &Path,
) -> Result<()> {
    // Without source workflows a run cannot tell "retired" from "never scanned",
    // and removing every marked file would wipe each target's synced workflows.
    if workflows.is_empty() {
        warn!("no source workflows found, skipping orphan removal");
        return Ok(());
    }

    for repository in repositories {
        let expected = expected_workflows(workflows, repository);

        let directory = targets
            .join(repository.owner.as_str())
            .join(repository.repository.as_str())
            .join(".github/workflows");
        for orphan in find_orphans(&directory, &expected).await? {
            tokio::fs::remove_file(&orphan)
                .await
                .with_context(|| format!("removing orphaned workflow {}", orphan.display()))?;
            info!(path = %orphan.display(), "orphaned workflow removed");
        }
    }

    Ok(())
}

// List generated workflows in a checkout that the source no longer writes there.
async fn find_orphans(
    directory: &Path,
    expected: &HashSet<&str>,
) -> Result<Vec<std::path::PathBuf>> {
    let mut entries = match tokio::fs::read_dir(directory).await {
        Ok(entries) => entries,
        // A target repository need not have a workflow directory yet.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("reading target workflows {}", directory.display()));
        }
    };

    let mut orphans = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .context("reading next target workflow")?
    {
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| expected.contains(name))
        {
            continue;
        }
        let path = entry.path();
        if !entry.file_type().await.context("get file type")?.is_file() {
            continue;
        }
        // Only files this tool generated are eligible for removal.
        let contents = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("reading target workflow {}", path.display()))?;
        if contents.starts_with(MARKER_PREFIX) {
            debug!(path = %path.display(), "found orphaned workflow");
            orphans.push(path);
        }
    }

    Ok(orphans)
}

/// Return checked-out repositories whose working trees need a pull request.
pub fn repositories_needing_pr(repositories: &[Repo], targets: &Path) -> Result<Vec<Repo>> {
    let mut repositories = repositories.to_vec();
    repositories.sort_by(|left, right| {
        left.owner
            .as_str()
            .cmp(right.owner.as_str())
            .then_with(|| left.repository.as_str().cmp(right.repository.as_str()))
    });

    let mut changed = Vec::new();
    for repository in repositories {
        let path = targets
            .join(repository.owner.as_str())
            .join(repository.repository.as_str());
        let git_repository = Repository::open(&path)
            .with_context(|| format!("opening checked-out repository {}", path.display()))?;
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .exclude_submodules(true);
        if !git_repository.statuses(Some(&mut options))?.is_empty() {
            changed.push(repository);
        }
    }

    Ok(changed)
}

// Clone a target repository, or refresh its existing checkout from origin.
fn checkout_repository(
    repository: Repo,
    targets: &Path,
    token: &FullyRedacted<GithubToken>,
) -> Result<()> {
    let owner = repository.owner.as_str();
    let name = repository.repository.as_str();
    let path = targets.join(owner).join(name);
    std::fs::create_dir_all(
        path.parent()
            .context("repository checkout path has no parent")?,
    )?;

    if path.exists() {
        info!(repository = %format_args!("{owner}/{name}"), path = %path.display(), "refreshing repository");
        refresh_repository(&path, token).with_context(|| format!("refreshing {owner}/{name}"))?;
        info!(repository = %format_args!("{owner}/{name}"), "repository refreshed");
    } else {
        info!(repository = %format_args!("{owner}/{name}"), path = %path.display(), "cloning repository");
        RepoBuilder::new()
            .fetch_options(fetch_options(token))
            .clone(&format!("https://github.com/{owner}/{name}.git"), &path)
            .with_context(|| format!("cloning {owner}/{name}"))?;
        info!(repository = %format_args!("{owner}/{name}"), "repository cloned");
    }

    Ok(())
}

// Build authenticated git fetch options for the GitHub token.
fn fetch_options(token: &GithubToken) -> FetchOptions<'_> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token.expose_secret()));

    let mut options = FetchOptions::new();
    options.remote_callbacks(callbacks);
    options
}

// Reset an existing checkout to origin's default branch and latest commit.
fn refresh_repository(path: &std::path::Path, token: &GithubToken) -> Result<()> {
    let repository = Repository::open(path).context("opening existing repository")?;
    let mut options = fetch_options(token);
    let mut remote = repository
        .find_remote("origin")
        .context("finding origin remote")?;
    remote
        .fetch(
            &["refs/heads/*:refs/remotes/origin/*"],
            Some(&mut options),
            None,
        )
        .context("fetching origin")?;
    let branch = remote
        .default_branch()
        .context("reading origin default branch")?
        .as_str()
        .context("origin default branch is not UTF-8")?
        .strip_prefix("refs/heads/")
        .context("origin default branch has unexpected format")?
        .to_owned();

    let remote_reference = repository
        .find_reference(&format!("refs/remotes/origin/{branch}"))
        .context("finding fetched branch")?;
    let target = remote_reference
        .peel_to_commit()
        .context("resolving fetched branch")?;
    // Discard all local, untracked, and ignored changes before applying source workflows.
    let mut checkout = CheckoutBuilder::new();
    checkout.force().remove_untracked(true).remove_ignored(true);
    repository
        .reset(target.as_object(), ResetType::Hard, Some(&mut checkout))
        .context("resetting working tree")?;
    let current_branch = repository.head()?.shorthand()?.to_owned();
    if current_branch != branch {
        repository
            .branch(&branch, &target, true)
            .context("updating local default branch")?;
    }
    repository
        .set_head(&format!("refs/heads/{branch}"))
        .context("checking out origin default branch")?;
    repository
        .checkout_head(Some(&mut checkout))
        .context("cleaning working tree")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, fs, path::Path, time::SystemTime};

    use anyhow::{Context, Result};
    use git2::{Repository, Signature};
    use non_empty_string::NonEmptyString;

    use super::{refresh_repository, remove_orphans, repositories_needing_pr};
    use crate::shared::{GithubToken, Repo, SyncWorkflow};

    #[test]
    fn repositories_needing_pr_includes_untracked_changes() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("apix-sync-pr-{}-{unique}", std::process::id()));
        let source_path = root.join("source");
        let targets = root.join("targets");
        let target_path = targets.join("owner/repository");

        let source = Repository::init(&source_path)?;
        commit(&source, "version one")?;
        Repository::clone(source_path.to_string_lossy().as_ref(), &target_path)?;
        fs::write(target_path.join("new.txt"), "change")?;

        let workflow = SyncWorkflow {
            file_name: OsString::from("workflow.yml"),
            // Safe: both values are non-empty test literals.
            sync: vec![Repo::new(
                NonEmptyString::new("owner".to_string()).expect("non-empty owner"),
                NonEmptyString::new("repository".to_string()).expect("non-empty repository"),
            )],
            contents: String::new(),
        };
        let changed = repositories_needing_pr(&workflow.sync.clone(), &targets)?;

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].owner.as_str(), "owner");
        assert_eq!(changed[0].repository.as_str(), "repository");

        drop(source);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn refresh_repository_discards_local_changes() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("apix-sync-{}-{unique}", std::process::id()));
        let source_path = root.join("source");
        let target_path = root.join("target");

        let source = Repository::init(&source_path)?;
        commit(&source, "version one")?;
        let target = Repository::clone(source_path.to_string_lossy().as_ref(), &target_path)?;
        drop(target);

        commit(&source, "version two")?;
        fs::write(target_path.join("tracked.txt"), "local change")?;
        fs::write(target_path.join("untracked.txt"), "remove me")?;
        fs::write(target_path.join("ignored.txt"), "remove me too")?;

        let token = "test-token".parse::<GithubToken>()?;
        refresh_repository(&target_path, &token)?;

        assert_eq!(
            fs::read_to_string(target_path.join("tracked.txt"))?,
            "version two"
        );
        assert!(!target_path.join("untracked.txt").exists());
        assert!(!target_path.join("ignored.txt").exists());

        drop(source);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    // Build a target checkout containing the given (name, contents) workflows.
    fn target_with_workflows(label: &str, files: &[(&str, &str)]) -> Result<std::path::PathBuf> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let targets =
            std::env::temp_dir().join(format!("apix-{label}-{}-{unique}", std::process::id()));
        let workflows = targets.join("owner/repository/.github/workflows");
        fs::create_dir_all(&workflows)?;
        for (name, contents) in files {
            fs::write(workflows.join(name), contents)?;
        }
        Ok(targets)
    }

    fn repo(name: &str) -> Repo {
        // Safe: both values are non-empty test literals.
        Repo::new(
            NonEmptyString::new("owner".to_string()).expect("non-empty owner"),
            NonEmptyString::new(name.to_string()).expect("non-empty repository"),
        )
    }

    fn workflow(file_name: &str, targets: &[&str]) -> SyncWorkflow {
        SyncWorkflow {
            file_name: OsString::from(file_name),
            sync: targets.iter().map(|target| repo(target)).collect(),
            contents: String::new(),
        }
    }

    #[tokio::test]
    async fn remove_orphans_removes_workflows_no_longer_targeted() -> Result<()> {
        let marker = super::MARKER_PREFIX;
        let targets = target_with_workflows(
            "orphan",
            &[
                ("kept.yaml", &format!("{marker}kept.yaml\nname: kept")),
                ("retired.yaml", &format!("{marker}retired.yaml\nname: gone")),
                // Owned by the target repository, so it must survive.
                ("local.yaml", "name: local"),
            ],
        )?;
        let workflows = targets.join("owner/repository/.github/workflows");

        remove_orphans(
            &[workflow("kept.yaml", &["repository"])],
            &[repo("repository")],
            &targets,
        )
        .await?;

        assert!(workflows.join("kept.yaml").exists());
        assert!(workflows.join("local.yaml").exists(), "unmarked file kept");
        assert!(!workflows.join("retired.yaml").exists());

        fs::remove_dir_all(targets)?;
        Ok(())
    }

    #[tokio::test]
    async fn remove_orphans_removes_workflow_dropped_for_one_target() -> Result<()> {
        let marker = super::MARKER_PREFIX;
        let targets = target_with_workflows(
            "orphan-header",
            &[("shared.yaml", &format!("{marker}shared.yaml\nname: shared"))],
        )?;
        let workflows = targets.join("owner/repository/.github/workflows");

        // The workflow still syncs, but no longer to this repository. The repository stays
        // in the target set because another workflow still writes to it.
        remove_orphans(
            &[
                workflow("kept.yaml", &["repository"]),
                workflow("shared.yaml", &["other"]),
            ],
            &[repo("repository")],
            &targets,
        )
        .await?;

        assert!(!workflows.join("shared.yaml").exists());

        fs::remove_dir_all(targets)?;
        Ok(())
    }

    #[tokio::test]
    async fn remove_orphans_skips_when_no_source_workflows() -> Result<()> {
        let marker = super::MARKER_PREFIX;
        let targets = target_with_workflows(
            "orphan-empty",
            &[("synced.yaml", &format!("{marker}synced.yaml\nname: synced"))],
        )?;
        let workflows = targets.join("owner/repository/.github/workflows");

        // A misconfigured run must not wipe every synced workflow.
        remove_orphans(&[], &[repo("repository")], &targets).await?;

        assert!(workflows.join("synced.yaml").exists());

        fs::remove_dir_all(targets)?;
        Ok(())
    }

    fn commit(repository: &Repository, contents: &str) -> Result<()> {
        let workdir = repository
            .workdir()
            .context("repository has no working tree")?;
        fs::write(workdir.join("tracked.txt"), contents)?;
        fs::write(workdir.join(".gitignore"), "ignored.txt\n")?;

        let mut index = repository.index()?;
        index.add_path(Path::new("tracked.txt"))?;
        index.add_path(Path::new(".gitignore"))?;
        index.write()?;
        let tree = repository.find_tree(index.write_tree()?)?;
        let signature = Signature::now("test", "test@example.com")?;
        let parent = repository
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();
        repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "test",
            &tree,
            &parents,
        )?;
        Ok(())
    }
}
