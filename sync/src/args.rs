use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use redacted::FullyRedacted;

use crate::shared::{GithubToken, GithubTokenError};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(long, env = "GH_TOKEN")]
    #[arg(value_parser = parse_github_token)]
    pub token: Option<FullyRedacted<GithubToken>>,

    #[arg(long, env = "WORKFLOW_DIRECTORY")]
    pub workflows_directory: Option<PathBuf>,

    #[arg(long, env = "SYNC_OWNER")]
    pub owner: Option<String>,

    /// Write created pull requests as JSON to a file.
    #[arg(long, value_name = "FILE")]
    pub json: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print owners declared by syncable workflows as a JSON array.
    Owners,

    /// Render a Markdown summary from created pull request JSON files.
    Summary {
        #[arg(long, default_value = "sync-prs")]
        directory: PathBuf,
    },
}

// Parse and redact the GitHub token supplied through the CLI or environment.
fn parse_github_token(value: &str) -> Result<FullyRedacted<GithubToken>, GithubTokenError> {
    value.parse().map(FullyRedacted::new)
}

impl Args {
    /// Return configured workflow directory, or `.github/workflows` in current directory.
    pub fn workflows_directory_or_default(&self) -> Result<PathBuf> {
        if let Some(workflows_directory) = &self.workflows_directory {
            return Ok(workflows_directory.clone());
        };

        Ok(env::current_dir()
            .context("reading working directory")?
            .join(".github/workflows"))
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Args;

    #[test]
    fn json_accepts_output_path() {
        // Safe: arguments are compile-time test literals.
        let args =
            Args::try_parse_from(["sync", "--json", "results.json"]).expect("valid CLI arguments");

        assert_eq!(
            args.json.as_deref(),
            Some(std::path::Path::new("results.json"))
        );
    }
}
