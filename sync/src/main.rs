use anyhow::Result;
use clap::Parser;

use crate::args::Args;

mod apix_action;
mod args;
mod shared;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let workflows_directory = args.workflows_directory_or_default();
    let _workflows = apix_action::scan(workflows_directory).await;

    Ok(())
}
