use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use git2::{
    Cred, FetchOptions, RemoteCallbacks, Repository, ResetType,
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
    let branch = {
        let head = repository.head().context("reading checked-out branch")?;
        head.shorthand()
            .context("existing repository has detached HEAD")?
            .to_owned()
    };

    let mut options = fetch_options(token);
    repository
        .find_remote("origin")
        .context("finding origin remote")?
        .fetch(&[&branch], Some(&mut options), None)
        .context("fetching origin")?;

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
    repository
        .checkout_head(Some(&mut checkout))
        .context("cleaning working tree")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, time::SystemTime};

    use anyhow::{Context, Result};
    use git2::{Repository, Signature};

    use super::refresh_repository;
    use crate::shared::GithubToken;

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
