use std::{
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use git2::{Repository, Status, StatusOptions};
use octocrab::{Octocrab, models::IssueState, params, params::repos::Reference};
use redacted::FullyRedacted;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::shared::{GithubToken, Repo};

const LABEL: &str = "apix-action";
const TITLE: &str = "ci: apix-action updates";

// Commits created through this mutation are signed by GitHub, so they show up as verified.
const CREATE_COMMIT: &str = "
mutation (
  $repository: String!,
  $branch: String!,
  $expectedHeadOid: GitObjectID!,
  $message: String!,
  $additions: [FileAddition!]!,
  $deletions: [FileDeletion!]!
) {
  createCommitOnBranch(input: {
    branch: {repositoryNameWithOwner: $repository, branchName: $branch},
    expectedHeadOid: $expectedHeadOid,
    message: {headline: $message},
    fileChanges: {additions: $additions, deletions: $deletions}
  }) {
    commit {
      oid
      signature { wasSignedByGitHub }
    }
  }
}";

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
        let base = publish_changes(&github, &repository, &path, &branch)
            .await
            .with_context(|| {
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

// List the changed paths of a checkout, along with how each one changed.
fn statuses(path: &Path) -> Result<Vec<(String, Status)>> {
    let repository = Repository::open(path).context("opening repository")?;
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true);

    repository
        .statuses(Some(&mut options))?
        .iter()
        .map(|status| {
            Ok((
                status.path().context("reading changed path")?.to_owned(),
                status.status(),
            ))
        })
        .collect()
}

// Read statuses before publish_changes commits them.
fn changed_workflows(path: &Path) -> Result<Vec<String>> {
    let mut actions = statuses(path)?
        .into_iter()
        .filter_map(|(path, _)| {
            path.strip_prefix(".github/workflows/")
                .map(|path| path.to_owned())
        })
        .collect::<Vec<_>>();
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

// Working tree changes, encoded the way `createCommitOnBranch` expects them.
#[derive(Debug, Default, Serialize)]
struct FileChanges {
    additions: Vec<FileAddition>,
    deletions: Vec<FileDeletion>,
}

#[derive(Debug, Serialize)]
struct FileAddition {
    path: String,
    /// Base64-encoded file contents, as required by the GraphQL schema.
    contents: String,
}

#[derive(Debug, Serialize)]
struct FileDeletion {
    path: String,
}

#[derive(Debug, Deserialize)]
struct CreateCommit {
    #[serde(rename = "createCommitOnBranch")]
    create_commit_on_branch: CreatedCommit,
}

#[derive(Debug, Deserialize)]
struct CreatedCommit {
    commit: CommitDetails,
}

#[derive(Debug, Deserialize)]
struct CommitDetails {
    oid: String,
    signature: Option<CommitSignature>,
}

#[derive(Debug, Deserialize)]
struct CommitSignature {
    #[serde(rename = "wasSignedByGitHub")]
    was_signed_by_github: bool,
}

// Read the current branch and commit of a checkout.
fn head(path: &Path) -> Result<(String, String)> {
    let repository = Repository::open(path).context("opening repository")?;
    let reference = repository.head().context("reading repository HEAD")?;
    let branch = reference
        .shorthand()
        .context("repository has detached HEAD")?
        .to_owned();
    let commit = reference
        .peel_to_commit()
        .context("resolving repository HEAD")?
        .id()
        .to_string();

    Ok((branch, commit))
}

// Turn the working tree changes into a GraphQL file change set.
fn file_changes(path: &Path) -> Result<FileChanges> {
    let mut changes = FileChanges::default();
    for (file, status) in statuses(path)? {
        if status.intersects(Status::WT_DELETED | Status::INDEX_DELETED) {
            changes.deletions.push(FileDeletion { path: file });
            continue;
        }

        let contents = fs::read(path.join(&file))
            .with_context(|| format!("reading changed file {file}"))
            .map(|contents| STANDARD.encode(contents))?;
        changes.additions.push(FileAddition {
            path: file,
            contents,
        });
    }
    changes
        .additions
        .sort_by(|left, right| left.path.cmp(&right.path));
    changes
        .deletions
        .sort_by(|left, right| left.path.cmp(&right.path));

    Ok(changes)
}

// Commit generated files remotely so that GitHub signs the commit, and return the base branch.
async fn publish_changes(
    github: &Octocrab,
    repository: &Repo,
    path: &Path,
    branch: &str,
) -> Result<String> {
    let owner = repository.owner.as_str();
    let name = repository.repository.as_str();
    let (base, expected_head) = head(path)?;
    let changes = file_changes(path)?;
    if changes.additions.is_empty() && changes.deletions.is_empty() {
        bail!("no changes to commit in {owner}/{name}");
    }

    // `createCommitOnBranch` only commits onto an existing branch, so branch off the base first.
    github
        .repos(owner, name)
        .create_ref(&Reference::Branch(branch.to_owned()), &expected_head)
        .await
        .with_context(|| format!("creating branch {branch} in {owner}/{name}"))?;

    let commit: CreateCommit = github
        .graphql(&serde_json::json!({
            "query": CREATE_COMMIT,
            "variables": {
                "repository": format!("{owner}/{name}"),
                "branch": branch,
                "expectedHeadOid": expected_head,
                "message": TITLE,
                "additions": changes.additions,
                "deletions": changes.deletions,
            }
        }))
        .await
        .with_context(|| format!("creating commit on {branch} in {owner}/{name}"))?;
    let commit = commit.create_commit_on_branch.commit;

    // A commit that GitHub did not sign would show up as unverified, which defeats the purpose.
    if !commit
        .signature
        .is_some_and(|signature| signature.was_signed_by_github)
    {
        bail!(
            "commit {} in {owner}/{name} was not signed by GitHub",
            commit.oid
        );
    }
    info!(repository = %format_args!("{owner}/{name}"), commit = %commit.oid, "signed commit created");

    Ok(base)
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use anyhow::Result;

    use super::{
        CreatedPullRequest, changed_workflows, description, file_changes, summary_directory,
    };

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

    #[test]
    fn file_changes_encodes_additions_and_lists_deletions() -> Result<()> {
        let unique = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("apix-changes-{}-{unique}", std::process::id()));
        let workflows = root.join(".github/workflows");
        fs::create_dir_all(&workflows)?;
        fs::write(workflows.join("removed.yaml"), "removed")?;

        // Commit one workflow so that removing it later shows up as a deletion.
        let repository = git2::Repository::init(&root)?;
        let mut index = repository.index()?;
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree = repository.find_tree(index.write_tree()?)?;
        let signature = git2::Signature::now("test", "test@example.com")?;
        repository.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])?;

        fs::remove_file(workflows.join("removed.yaml"))?;
        fs::write(workflows.join("added.yaml"), "added")?;

        let changes = file_changes(&root)?;
        assert_eq!(
            changes
                .additions
                .iter()
                .map(|addition| (addition.path.as_str(), addition.contents.as_str()))
                .collect::<Vec<_>>(),
            // "added" base64-encoded.
            vec![(".github/workflows/added.yaml", "YWRkZWQ=")]
        );
        assert_eq!(
            changes
                .deletions
                .iter()
                .map(|deletion| deletion.path.as_str())
                .collect::<Vec<_>>(),
            vec![".github/workflows/removed.yaml"]
        );

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
