use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::args::Args;

mod args;
mod labels;
mod scan;
mod shared;
mod sync;
mod sync_entry;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();

    let workflows_directory = args.workflows_directory_or_default();
    info!(directory = %workflows_directory.display(), "scanning workflows");
    let workflows = scan::scan(workflows_directory).await?;
    info!(workflows = workflows.len(), "found syncable workflows");

    let targets_directory = env::current_dir()
        .context("reading working directory")?
        .join("targets");

    sync::sync(&workflows, targets_directory.clone(), args.token.clone()).await?;
    info!("sync complete");

    labels::ensure(&workflows, args.token).await?;
    info!("labels are present in all repositories");

    let repositories_needing_pr = sync::repositories_needing_pr(&workflows, &targets_directory)?;
    info!(
        repositories = ?repositories_needing_pr,
        "repositories needing pull requests"
    );

    Ok(())
}
