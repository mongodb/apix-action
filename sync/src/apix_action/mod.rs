use anyhow::{Context, Result};

use std::path::PathBuf;

use crate::shared::Repo;

mod sync_entry;

/// Scans workflows that contain at least one valid sync header.
pub async fn scan(directory: PathBuf) -> Result<Vec<SyncWorkflow>> {
    let mut entries = tokio::fs::read_dir(directory)
        .await
        .context("reading workflow directory")?;

    let mut workflows = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .context("reading next workflow entry")?
    {
        let file_type = entry.file_type().await.context("get file type")?;

        let path = entry.path();
        if !file_type.is_file() || path.extension().is_none_or(|extension| extension != "yaml") {
            continue;
        }

        let contents = tokio::fs::read_to_string(path)
            .await
            .context("read workflow contents")?;

        let workflow = parse_workflow(&contents);
        workflows.push(workflow);
    }

    Ok(workflows)
}

fn parse_workflow(contents: &str) -> SyncWorkflow {
    let mut sync = Vec::new();
    let filtered_lines: Vec<_> = contents
        .lines()
        .filter(|line| match line.parse::<sync_entry::SyncEntry>() {
            Ok(entry) => {
                sync.push(entry.repo);
                false
            }
            Err(_) => true,
        })
        .collect();

    let contents = filtered_lines.join("\n").trim().to_string();

    SyncWorkflow { sync, contents }
}

/// Workflow that has to be synced.
pub struct SyncWorkflow {
    /// Sync targets.
    pub sync: Vec<Repo>,
    /// Workflow contents without sync headers.
    pub contents: String,
}

#[cfg(test)]
mod tests {
    use super::parse_workflow;
    use non_empty_string::NonEmptyString;

    #[test]
    fn parse_workflow_removes_valid_sync_headers() {
        let contents = "name: test\n# sync -> mongodb/first\n# sync -> mongodb/second\njobs:\n";

        let workflow = parse_workflow(contents);

        assert_eq!(workflow.sync.len(), 2);
        assert_eq!(
            workflow.sync[0].owner,
            NonEmptyString::new("mongodb".to_string()).unwrap()
        );
        assert_eq!(
            workflow.sync[0].repository,
            NonEmptyString::new("first".to_string()).unwrap()
        );
        assert_eq!(workflow.contents, "name: test\njobs:");
    }

    #[test]
    fn parse_workflow_keeps_invalid_sync_headers() {
        let contents = "# sync -> invalid\n# not a sync header\n";

        let workflow = parse_workflow(contents);

        assert!(workflow.sync.is_empty());
        assert_eq!(workflow.contents, contents.trim());
    }
}
