use std::env;
use std::path::PathBuf;

use clap::Parser;
use redacted::FullyRedacted;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(long, env = "GH_TOKEN")]
    pub token: FullyRedacted<String>,

    #[arg(long, env = "WORKFLOWA_DIRECTORY")]
    pub workflows_directory: Option<PathBuf>,
}

impl Args {
    pub fn workflows_directory_or_default(&self) -> PathBuf {
        if let Some(workflows_directory) = &self.workflows_directory {
            return workflows_directory.clone();
        };

        // should never fail
        let working_directory = env::current_dir().expect("valid work directory");
        working_directory.join(".github/workflows")
    }
}
