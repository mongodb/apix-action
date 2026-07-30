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
use tracing::{debug, error, info};

use crate::shared::{GithubToken, Repo, SyncWorkflow};

/// Checks out sync targets and writes their applicable workflows.
pub async fn sync(
    workflows: &[SyncWorkflow],
    targets: PathBuf,
    token: FullyRedacted<GithubToken>,
) -> Result<()> {
    let repositories: HashSet<_> = workflows
        .iter()
        .flat_map(|workflow| workflow.sync.iter().cloned())
        .collect();
    info!(
        repositories = repositories.len(),
        "checking out repositories"
    );
    let token = Arc::new(token);
    let mut tasks = JoinSet::new();
    for repository in repositories {
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

    Ok(())
}

/// Returns checked-out repositories with changes that need a pull request.
pub fn repositories_needing_pr(workflows: &[SyncWorkflow], targets: &Path) -> Result<Vec<Repo>> {
    let mut repositories: Vec<_> = workflows
        .iter()
        .flat_map(|workflow| workflow.sync.iter().cloned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
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

fn fetch_options(token: &GithubToken) -> FetchOptions<'_> {
    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token.expose_secret()));

    let mut options = FetchOptions::new();
    options.remote_callbacks(callbacks);
    options
}

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

    use super::{refresh_repository, repositories_needing_pr};
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
        let changed = repositories_needing_pr(&[workflow], &targets)?;

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
