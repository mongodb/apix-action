use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::args::{Args, Command};

mod args;
mod installation;
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
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(tracing::Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();
    let workflows_directory = args.workflows_directory_or_default()?;

    match args.command {
        Some(Command::Owners) => {
            // Listing installations covers owners whose workflows are all retired, which
            // the sync headers no longer mention.
            let app_id = args.app_id.context("app ID is required to list owners")?;
            let private_key = args
                .private_key
                .context("app private key is required to list owners")?;
            let owners = installation::owners(app_id, &private_key).await?;
            println!("{}", serde_json::to_string(&owners)?);
            return Ok(());
        }
        Some(Command::Summary { directory }) => {
            print!("{}", prs::summary_directory(directory)?);
            return Ok(());
        }
        None => {}
    }

    // Find source workflows that declare repositories to sync.
    info!(directory = %workflows_directory.display(), "scanning workflows");

    let owner = args.owner.as_deref();
    let scanned = scan::scan(workflows_directory).await?;
    // Checked before the owner filter below. An owner with no workflows left is
    // legitimate: everything it had was retired, and its copies should be swept. A source
    // tree with none is indistinguishable from a wrong WORKFLOW_DIRECTORY, and sweeping on
    // that would wipe every synced workflow.
    if scanned.is_empty() {
        anyhow::bail!("no syncable workflows found, refusing to sync");
    }

    let workflows = scanned
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

    let token = args.token.context("GitHub token is required for sync")?;

    let repositories = installation::repositories_to_sync(&token, owner, &workflows).await?;
    info!(repositories = repositories.len(), "repositories to sync");

    // Check out all target repositories and write their applicable workflows.
    sync::sync(
        &workflows,
        &repositories,
        targets_directory.clone(),
        token.clone(),
    )
    .await?;
    info!("sync complete");

    // Ensure every target has `apix-action`, which the PR phase uses to find old generated PRs.
    labels::ensure(&workflows, token.clone()).await?;
    info!("labels are present in all repositories");

    // Only create pull requests for repositories whose working tree changed.
    let repositories_needing_pr = sync::repositories_needing_pr(&repositories, &targets_directory)?;
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
    let pull_requests = prs::create(repositories_needing_pr, targets_directory, token).await?;
    if let Some(path) = args.json {
        let json = serde_json::to_string_pretty(&pull_requests)?;
        std::fs::write(&path, json)
            .with_context(|| format!("writing pull request results to {}", path.display()))?;
    }

    Ok(())
}
