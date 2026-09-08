use std::collections::{BTreeSet, HashSet};

use anyhow::{Context, Result};
use octocrab::{Octocrab, models::AppId};
use redacted::FullyRedacted;
use serde::Deserialize;
use tracing::{debug, info};

use crate::{
    scan::MARKER_PREFIX,
    shared::{AppPrivateKey, Repo, SyncWorkflow, expected_workflows},
};

const WORKFLOWS_PATH: &str = ".github/workflows";
const PER_PAGE: u8 = 100;

#[derive(Deserialize)]
struct InstallationRepositories {
    repositories: Vec<InstallationRepository>,
}

#[derive(Deserialize)]
struct InstallationRepository {
    name: String,
    owner: InstallationOwner,
}

#[derive(Deserialize)]
struct InstallationOwner {
    login: String,
}

/// List every owner the app is installed on.
///
/// Authenticates as the app itself rather than as one installation, so owners whose
/// workflows are all retired are still swept.
pub async fn owners(
    app_id: u64,
    private_key: &FullyRedacted<AppPrivateKey>,
) -> Result<BTreeSet<String>> {
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.expose_secret().as_bytes())
        .context("parsing app private key")?;
    let github = Octocrab::builder()
        .app(AppId(app_id), key)
        .build()
        .context("creating GitHub app client")?;

    let installations = github
        .apps()
        .installations()
        .per_page(PER_PAGE)
        .send()
        .await
        .context("listing app installations")?;
    let installations = github
        .all_pages(installations)
        .await
        .context("listing remaining app installations")?;

    let owners: BTreeSet<_> = installations
        .into_iter()
        .map(|installation| installation.account.login)
        .collect();
    info!(owners = owners.len(), "listed app installation owners");

    Ok(owners)
}

/// List repositories the app installation can reach, optionally limited to one owner.
pub async fn repositories(github: &Octocrab, owner: Option<&str>) -> Result<Vec<Repo>> {
    let mut repositories = Vec::new();
    let mut page = 1u32;
    loop {
        let response: InstallationRepositories = github
            .get(
                format!("/installation/repositories?per_page={PER_PAGE}&page={page}"),
                None::<&()>,
            )
            .await
            .context("listing installation repositories")?;
        let count = response.repositories.len();

        for repository in response.repositories {
            if owner.is_some_and(|owner| repository.owner.login != owner) {
                continue;
            }
            // Names come from GitHub, but they are joined onto checkout paths.
            let Ok(repo) = format!("{}/{}", repository.owner.login, repository.name).parse() else {
                debug!(
                    repository = %format_args!("{}/{}", repository.owner.login, repository.name),
                    "skipping repository with unsupported name"
                );
                continue;
            };
            repositories.push(repo);
        }

        if count < usize::from(PER_PAGE) {
            break;
        }
        page += 1;
    }

    info!(
        repositories = repositories.len(),
        "listed installation repositories"
    );
    Ok(repositories)
}

/// Find installation repositories holding synced workflows the source no longer sends them.
///
/// Reads each repository's workflow directory over the API so that only repositories
/// actually needing a change are cloned.
pub async fn repositories_with_orphans(
    github: &Octocrab,
    repositories: &[Repo],
    workflows: &[SyncWorkflow],
) -> Result<Vec<Repo>> {
    let mut found = Vec::new();
    for repository in repositories {
        let expected = expected_workflows(workflows, repository);
        if has_orphan(github, repository, &expected).await? {
            info!(
                repository = %format_args!("{}/{}", repository.owner, repository.repository),
                "found orphaned workflows"
            );
            found.push(repository.clone());
        }
    }

    Ok(found)
}

// Report whether a repository holds a generated workflow the source no longer writes there.
async fn has_orphan(
    github: &Octocrab,
    repository: &Repo,
    expected: &HashSet<&str>,
) -> Result<bool> {
    let repos = github.repos(repository.owner.as_str(), repository.repository.as_str());
    let contents = match repos.get_content().path(WORKFLOWS_PATH).send().await {
        Ok(contents) => contents,
        // A repository need not have any workflows.
        Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
            return Ok(false);
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "listing workflows in {}/{}",
                    repository.owner, repository.repository
                )
            });
        }
    };

    for item in contents.items {
        if item.r#type != "file" || expected.contains(item.name.as_str()) {
            continue;
        }
        if !(item.name.ends_with(".yaml") || item.name.ends_with(".yml")) {
            continue;
        }

        // Only files this tool generated count, so fetch just the candidates.
        let file = repos
            .get_content()
            .path(&item.path)
            .send()
            .await
            .with_context(|| {
                format!(
                    "reading {} in {}/{}",
                    item.path, repository.owner, repository.repository
                )
            })?;
        let is_generated = file
            .items
            .first()
            .and_then(|file| file.decoded_content())
            .is_some_and(|contents| contents.starts_with(MARKER_PREFIX));
        if is_generated {
            debug!(
                repository = %format_args!("{}/{}", repository.owner, repository.repository),
                workflow = %item.name,
                "orphaned workflow detected"
            );
            return Ok(true);
        }
    }

    Ok(false)
}
