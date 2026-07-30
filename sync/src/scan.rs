use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};
use tracing::debug;

use crate::{shared::SyncWorkflow, sync_entry::SyncEntry};

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
        if !file_type.is_file()
            || !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("yaml" | "yml")
            )
        {
            debug!(path = %path.display(), "skipping non-workflow entry");
            continue;
        }

        let contents = tokio::fs::read_to_string(&path)
            .await
            .context("read workflow contents")?;

        let workflow = parse_workflow(entry.file_name(), &contents);
        if !workflow.sync.is_empty() {
            debug!(
                path = %path.display(),
                targets = workflow.sync.len(),
                "found syncable workflow"
            );
            workflows.push(workflow);
        }
    }

    Ok(workflows)
}

fn parse_workflow(file_name: OsString, contents: &str) -> SyncWorkflow {
    let mut sync = Vec::new();
    let filtered_lines: Vec<_> = contents
        .lines()
        .filter(|line| match line.parse::<SyncEntry>() {
            Ok(entry) => {
                sync.push(entry.repo);
                false
            }
            Err(_) => true,
        })
        .collect();

    let contents = filtered_lines.join("\n").trim().to_string();
    let contents = if sync.is_empty() {
        contents
    } else {
        format!(
            "# synced from apix-actions/.github/workflows/{}\n{contents}",
            file_name.to_string_lossy()
        )
    };

    SyncWorkflow {
        file_name,
        sync,
        contents,
    }
}

#[cfg(test)]
mod tests {
    use non_empty_string::NonEmptyString;

    use super::parse_workflow;

    #[test]
    fn parse_workflow_removes_valid_sync_headers() {
        let contents = "name: test\n# sync -> mongodb/first\n# sync -> mongodb/second\njobs:\n";

        let workflow = parse_workflow("test.yaml".into(), contents);

        assert_eq!(workflow.file_name, "test.yaml");
        assert_eq!(workflow.sync.len(), 2);
        assert_eq!(
            workflow.sync[0].owner,
            NonEmptyString::new("mongodb".to_string()).unwrap()
        );
        assert_eq!(
            workflow.sync[0].repository,
            NonEmptyString::new("first".to_string()).unwrap()
        );
        assert_eq!(
            workflow.contents,
            "# synced from apix-actions/.github/workflows/test.yaml\nname: test\njobs:"
        );
    }

    #[test]
    fn parse_workflow_keeps_invalid_sync_headers() {
        let contents = "# sync -> invalid\n# not a sync header\n";

        let workflow = parse_workflow("test.yaml".into(), contents);

        assert!(workflow.sync.is_empty());
        assert_eq!(workflow.contents, contents.trim());
    }

    #[test]
    fn parse_workflow_adds_comment_when_contents_are_empty() {
        let workflow = parse_workflow("test.yaml".into(), "# sync -> mongodb/test\n");

        assert_eq!(
            workflow.contents,
            "# synced from apix-actions/.github/workflows/test.yaml\n"
        );
    }
}
