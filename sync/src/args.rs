use std::env;
use std::path::PathBuf;

use clap::Parser;
use redacted::FullyRedacted;

use crate::shared::{GithubToken, GithubTokenError};

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(long, env = "GH_TOKEN")]
    #[arg(value_parser = parse_github_token)]
    pub token: FullyRedacted<GithubToken>,

    #[arg(long, env = "WORKFLOW_DIRECTORY")]
    pub workflows_directory: Option<PathBuf>,
}

// Parse and redact the GitHub token supplied through the CLI or environment.
fn parse_github_token(value: &str) -> Result<FullyRedacted<GithubToken>, GithubTokenError> {
    value.parse().map(FullyRedacted::new)
}

impl Args {
    /// Return configured workflow directory, or `.github/workflows` in current directory.
    pub fn workflows_directory_or_default(&self) -> PathBuf {
        if let Some(workflows_directory) = &self.workflows_directory {
            return workflows_directory.clone();
        };

        // should never fail
        let working_directory = env::current_dir().expect("valid work directory");
        working_directory.join(".github/workflows")
    }
}
