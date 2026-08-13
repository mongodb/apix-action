use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use git2::{
    Cred, IndexAddOption, PushOptions, RemoteCallbacks, Repository, Signature, StatusOptions,
};
use octocrab::{Octocrab, models::IssueState, params};
use redacted::FullyRedacted;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::shared::{GithubToken, Repo};

const LABEL: &str = "apix-action";
const TITLE: &str = "ci: apix-action updates";

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatedPullRequest {
    pub repository: String,
    pub url: String,
    pub files: Vec<String>,
}

/// Close old generated PRs, publish changes, and open replacement PRs.
pub async fn create(
    repositories: Vec<Repo>,
    targets: impl AsRef<Path>,
    token: FullyRedacted<GithubToken>,
) -> Result<Vec<CreatedPullRequest>> {
    let github = Octocrab::builder()
        .personal_token(token.expose_secret())
        .build()
        .context("creating GitHub client")?;

    // Close open PRs carrying `apix-action` before opening replacement PRs.
    for repository in &repositories {
        close_labeled_pull_requests(&github, repository).await?;
    }

    // One unique branch name is reused across target repositories.
    let branch = format!(
        "ci/api-action-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("reading system clock")?
            .as_nanos()
    );
    let mut pull_requests = Vec::new();

    for repository in repositories {
        let path = targets
            .as_ref()
            .join(repository.owner.as_str())
            .join(repository.repository.as_str());
        let actions = changed_workflows(&path).with_context(|| {
            format!(
                "finding changed workflows for {}/{}",
                repository.owner, repository.repository
            )
        })?;
        let base = publish_changes(&path, &branch, &token).with_context(|| {
            format!(
                "publishing changes for {}/{}",
                repository.owner, repository.repository
            )
        })?;

        let pull_request = github
            .pulls(repository.owner.as_str(), repository.repository.as_str())
            .create(TITLE, format!("{}:{branch}", repository.owner), &base)
            .body(description(&actions))
            .send()
            .await
            .with_context(|| {
                format!(
                    "creating pull request for {}/{}",
                    repository.owner, repository.repository
                )
            })?;

        github
            .issues(repository.owner.as_str(), repository.repository.as_str())
            .add_labels(pull_request.number, &[LABEL.to_string()])
            .await
            .with_context(|| {
                format!(
                    "labeling pull request for {}/{}",
                    repository.owner, repository.repository
                )
            })?;
        let url = format!(
            "https://github.com/{}/{}/pull/{}",
            repository.owner, repository.repository, pull_request.number
        );
        info!(url = %url, "pull request created");
        pull_requests.push(CreatedPullRequest {
            repository: format!("{}/{}", repository.owner, repository.repository),
            url,
            files: actions,
        });
    }

    Ok(pull_requests)
}

/// Render created pull request JSON files as one Markdown summary.
fn summary(files: &[std::path::PathBuf]) -> Result<String> {
    let mut pull_requests = Vec::new();
    for file in files {
        let path = file.as_path();
        let contents = fs::read_to_string(path)
            .with_context(|| format!("reading pull request results from {}", path.display()))?;
        pull_requests.extend(
            serde_json::from_str::<Vec<CreatedPullRequest>>(&contents)
                .with_context(|| format!("parsing pull request results from {}", path.display()))?,
        );
    }
    pull_requests.sort_by(|left, right| left.repository.cmp(&right.repository));

    let mut output = String::from("## Created pull requests\n\n");
    if pull_requests.is_empty() {
        output.push_str("No pull requests were created.\n");
        return Ok(output);
    }

    for pull_request in pull_requests {
        let files = pull_request
            .files
            .iter()
            .map(|file| format!("`{file}`"))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!(
            "- [`{}`]({}){}\n",
            pull_request.repository,
            pull_request.url,
            if files.is_empty() {
                String::new()
            } else {
                format!(": {files}")
            }
        ));
    }

    Ok(output)
}

/// Read all JSON result files in a directory and render one Markdown summary.
pub fn summary_directory(directory: impl AsRef<Path>) -> Result<String> {
    let directory = directory.as_ref();
    if !directory.exists() {
        return summary(&[]);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(directory)
        .with_context(|| format!("reading summary directory {}", directory.display()))?
    {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
    files.sort();
    summary(&files)
}

// Read statuses before publish_changes commits them.
fn changed_workflows(path: &Path) -> Result<Vec<String>> {
    let repository = Repository::open(path).context("opening repository")?;
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true);

    let statuses = repository.statuses(Some(&mut options))?;
    let mut actions = Vec::new();
    for status in statuses.iter() {
        let path = status.path().context("reading changed path")?;
        if let Some(path) = path.strip_prefix(".github/workflows/") {
            actions.push(path.to_owned());
        }
    }
    actions.sort();
    actions.dedup();
    Ok(actions)
}

// Build the Markdown body listing workflows changed for one repository.
fn description(actions: &[String]) -> String {
    let actions = actions
        .iter()
        .map(|action| format!("- `{action}`"))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "## 🤖 Automated sync\n\nThis pull request was autogenerated to sync this repository from `api-actions`.\n\n## Actions changed\n\n{actions}"
    )
}

// Close open pull requests carrying the `apix-action` label.
async fn close_labeled_pull_requests(github: &Octocrab, repository: &Repo) -> Result<()> {
    let pulls = github
        .pulls(repository.owner.as_str(), repository.repository.as_str())
        .list()
        .state(params::State::Open)
        .per_page(100)
        .send()
        .await
        .with_context(|| {
            format!(
                "listing pull requests in {}/{}",
                repository.owner, repository.repository
            )
        })?;
    let pulls = github.all_pages(pulls).await?;

    for pull_request in pulls {
        let has_label = pull_request
            .labels
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|label| label.name == LABEL);
        if !has_label {
            continue;
        }

        github
            .issues(repository.owner.as_str(), repository.repository.as_str())
            .update(pull_request.number)
            .state(IssueState::Closed)
            .send()
            .await
            .with_context(|| format!("closing pull request #{}", pull_request.number))?;
        info!(url = %format_args!("https://github.com/{}/{}/pull/{}", repository.owner, repository.repository, pull_request.number), "pull request closed");
    }

    Ok(())
}

// Commit current generated files on a branch and push it to origin.
fn publish_changes(
    path: &Path,
    branch: &str,
    token: &FullyRedacted<GithubToken>,
) -> Result<String> {
    let repository = Repository::open(path).context("opening repository")?;
    let base = repository
        .head()
        .context("reading repository HEAD")?
        .shorthand()
        .context("repository has detached HEAD")?
        .to_owned();
    let commit = repository
        .head()
        .context("reading repository HEAD")?
        .peel_to_commit()
        .context("resolving repository HEAD")?;

    // Commit generated files on a fresh branch, then push it for the PR.
    repository.branch(branch, &commit, false)?;
    repository.set_head(&format!("refs/heads/{branch}"))?;
    repository.checkout_head(None)?;

    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree = repository.find_tree(index.write_tree()?)?;
    let signature = Signature::now("apix-action", "apix-action@mongodb.com")?;
    repository.commit(
        Some("HEAD"),
        &signature,
        &signature,
        TITLE,
        &tree,
        &[&commit],
    )?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks
        .credentials(|_, _, _| Cred::userpass_plaintext("x-access-token", token.expose_secret()));
    callbacks.push_update_reference(|reference, status| {
        status.map_or(Ok(()), |status| {
            Err(git2::Error::from_str(&format!(
                "GitHub rejected push for {reference}: {status}"
            )))
        })
    });
    let mut options = PushOptions::new();
    options.remote_callbacks(callbacks);
    repository.find_remote("origin")?.push(
        &[format!("refs/heads/{branch}:refs/heads/{branch}")],
        Some(&mut options),
    )?;

    Ok(base)
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use anyhow::Result;

    use super::{CreatedPullRequest, changed_workflows, description, summary_directory};

    #[test]
    fn description_lists_changed_actions() {
        let actions = vec!["dependabot-auto-merge.yaml".to_string()];
        assert_eq!(
            description(&actions),
            "## 🤖 Automated sync\n\nThis pull request was autogenerated to sync this repository from `api-actions`.\n\n## Actions changed\n\n- `dependabot-auto-merge.yaml`"
        );
    }

    #[test]
    fn summary_lists_repositories_and_changed_files() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("apix-summary-{unique}"));
        fs::create_dir_all(&directory)?;
        let path = directory.join("prs.json");
        fs::write(
            &path,
            serde_json::to_string(&vec![CreatedPullRequest {
                repository: "mongodb/example".to_string(),
                url: "https://github.com/mongodb/example/pull/1".to_string(),
                files: vec!["ci.yaml".to_string()],
            }])?,
        )?;

        assert_eq!(
            summary_directory(&directory)?,
            "## Created pull requests\n\n- [`mongodb/example`](https://github.com/mongodb/example/pull/1): `ci.yaml`\n"
        );

        fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn changed_workflows_excludes_other_changed_files() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("apix-prs-{}-{unique}", std::process::id()));
        let workflows = root.join(".github/workflows");
        fs::create_dir_all(&workflows)?;
        fs::write(workflows.join("dependabot-auto-merge.yaml"), "workflow")?;
        fs::write(root.join("README.md"), "readme")?;
        git2::Repository::init(&root)?;

        assert_eq!(
            changed_workflows(&root)?,
            vec!["dependabot-auto-merge.yaml"]
        );

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
