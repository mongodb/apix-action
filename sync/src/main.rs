use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::args::{Args, Command};

mod args;
mod labels;
mod prs;
mod scan;
mod shared;
mod sync;
mod sync_entry;

#[tokio::main]
/// Scan source workflows, update target repositories, and open pull requests.
async fn main() -> Result<()> {
    // Set up logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();
    let workflows_directory = args.workflows_directory_or_default()?;

    if matches!(args.command, Some(Command::Owners)) {
        let owners = scan::scan(workflows_directory)
            .await?
            .into_iter()
            .flat_map(|workflow| workflow.sync.into_iter().map(|repository| repository.owner))
            .map(|owner| owner.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        println!("{}", serde_json::to_string(&owners)?);
        return Ok(());
    }

    // Find source workflows that declare repositories to sync.
    info!(directory = %workflows_directory.display(), "scanning workflows");

    let owner = args.owner.as_deref();
    let workflows = scan::scan(workflows_directory)
        .await?
        .into_iter()
        .filter_map(|mut workflow| {
            if let Some(owner) = owner {
                workflow
                    .sync
                    .retain(|repository| repository.owner.as_str() == owner);
            }
            (!workflow.sync.is_empty()).then_some(workflow)
        })
        .collect::<Vec<_>>();
    info!(workflows = workflows.len(), "found syncable workflows");

    let targets_directory = env::current_dir()
        .context("reading working directory")?
        .join("targets");

    // Check out all target repositories and write their applicable workflows.
    let token = args.token.context("GitHub token is required for sync")?;
    sync::sync(&workflows, targets_directory.clone(), token.clone()).await?;
    info!("sync complete");

    // Ensure every target has `apix-action`, which the PR phase uses to find old generated PRs.
    labels::ensure(&workflows, token.clone()).await?;
    info!("labels are present in all repositories");

    // Only create pull requests for repositories whose working tree changed.
    let repositories_needing_pr = sync::repositories_needing_pr(&workflows, &targets_directory)?;
    let repositories = repositories_needing_pr
        .iter()
        .map(|repository| format!("{}/{}", repository.owner, repository.repository))
        .collect::<Vec<_>>()
        .join(", ");
    info!(
        repositories = %repositories,
        "repositories needing pull requests"
    );

    // Close old `apix-action` PRs, then commit, push, and open replacement PRs.
    prs::create(repositories_needing_pr, targets_directory, token).await?;

    Ok(())
}
