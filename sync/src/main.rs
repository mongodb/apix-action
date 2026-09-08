use std::env;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    args::{Args, Command},
    shared::Repo,
};

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
            // the sync headers no longer mention. Without app credentials there is nothing
            // to ask, so fall back to the headers.
            let owners = match (args.app_id, &args.private_key) {
                (Some(app_id), Some(private_key)) => {
                    installation::owners(app_id, private_key).await?
                }
                _ => {
                    warn!("no app credentials, deriving owners from sync headers");
                    scan::scan(workflows_directory)
                        .await?
                        .into_iter()
                        .flat_map(|workflow| {
                            workflow.sync.into_iter().map(|repository| repository.owner)
                        })
                        .map(|owner| owner.to_string())
                        .collect()
                }
            };
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
    // An empty scan cannot tell "everything retired" from "wrong directory", and would
    // let the sweep wipe every synced workflow. An owner with no targets is fine; a
    // source tree with none is not.
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

    // Sweep the whole installation, so a repository dropped from every header is still
    // cleaned up. Only repositories with leftovers are added, keeping clones proportional
    // to actual changes rather than to installation size.
    let github = octocrab::Octocrab::builder()
        .personal_token(token.expose_secret())
        .build()
        .context("creating GitHub client")?;
    let installation = installation::repositories(&github, owner).await?;
    let orphaned = installation::repositories_with_orphans(&github, &installation, &workflows)
        .await
        .context("finding repositories with orphaned workflows")?;

    let mut repositories: Vec<Repo> = sync::target_repositories(&workflows)
        .into_iter()
        .chain(orphaned)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    repositories.sort_by(|left, right| {
        left.owner
            .as_str()
            .cmp(right.owner.as_str())
            .then_with(|| left.repository.as_str().cmp(right.repository.as_str()))
    });
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
