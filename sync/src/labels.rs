use std::collections::HashSet;

use anyhow::{Context, Result};
use octocrab::Octocrab;
use redacted::FullyRedacted;
use tracing::info;

use crate::shared::{GithubToken, Repo, SyncWorkflow};

const LABEL: &str = "apix-action";
const LABEL_COLOR: &str = "0366d6";

pub async fn ensure(workflows: &[SyncWorkflow], token: FullyRedacted<GithubToken>) -> Result<()> {
    let repositories: HashSet<_> = workflows
        .iter()
        .flat_map(|workflow| workflow.sync.iter().cloned())
        .collect();
    let github = Octocrab::builder()
        .personal_token(token.expose_secret())
        .build()
        .context("creating GitHub client")?;

    for repository in repositories {
        ensure_repository_label(&github, &repository).await?;
    }

    Ok(())
}

async fn ensure_repository_label(github: &Octocrab, repository: &Repo) -> Result<()> {
    let issues = github.issues(repository.owner.as_str(), repository.repository.as_str());
    let Err(error) = issues.get_label(LABEL).await else {
        info!(repository = %format_args!("{}/{}", repository.owner, repository.repository), "label exists");
        return Ok(());
    };

    if !matches!(&error, octocrab::Error::GitHub { source, .. } if source.status_code.as_u16() == 404)
    {
        return Err(error).with_context(|| {
            format!(
                "checking {LABEL} label in {}/{}",
                repository.owner, repository.repository
            )
        });
    }

    issues
        .create_label(LABEL, LABEL_COLOR, "")
        .await
        .with_context(|| {
            format!(
                "creating {LABEL} label in {}/{}",
                repository.owner, repository.repository
            )
        })?;
    info!(repository = %format_args!("{}/{}", repository.owner, repository.repository), "label created");

    Ok(())
}
