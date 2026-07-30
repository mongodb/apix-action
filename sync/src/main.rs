use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::args::Args;

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

    // Find source workflows that declare repositories to sync.
    info!(directory = %workflows_directory.display(), "scanning workflows");

    let workflows = scan::scan(workflows_directory).await?;
    info!(workflows = workflows.len(), "found syncable workflows");

    let targets_directory = env::current_dir()
        .context("reading working directory")?
        .join("targets");

    // Check out all target repositories and write their applicable workflows.
    sync::sync(&workflows, targets_directory.clone(), args.token.clone()).await?;
    info!("sync complete");

    // Ensure every target has `apix-action`, which the PR phase uses to find old generated PRs.
    labels::ensure(&workflows, args.token.clone()).await?;
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
    prs::create(
        &workflows,
        repositories_needing_pr,
        targets_directory,
        args.token,
    )
    .await?;

    Ok(())
}
