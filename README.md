# APIX Actions

A collection of reusable GitHub Actions for integrating with Jira and other common CI/CD tasks.

## Available Actions

### Jira Integration

- **[create-jira](./create-jira/README.md)**: Create a Jira issue with customizable fields
- **[find-jira](./find-jira/README.md)**: Find Jira issues using JQL queries
- **[transition-jira](./transition-jira/README.md)**: Transition Jira issues between states
- **[comment-jira](./comment-jira/README.md)**: Add a comment to a Jira issue
- **[update-jira](./update-jira/README.md)**: Update fields on one or more Jira issues (bulk-capable)
- **[get-jira-version](./get-jira-version/README.md)**: Look up a Jira project version (fix version) by name
- **[create-jira-version](./create-jira-version/README.md)**: Create a Jira project version (fix version)
- **[update-jira-version](./update-jira-version/README.md)**: Update a Jira project version (rename/release)
- **[link-jira](./link-jira/README.md)**: Link two Jira issues

### GitHub Integration

- **[token](./token/README.md)**: Get GitHub App token for authentication
- **[verify-changed-files](./verify-changed-files/README.md)**: Check which files have changed between git refs

## Usage

Each action has its own README with detailed usage instructions. Click the links above to learn more about each action.

## Sync Workflows

To sync all workflows to their configured repositories, start the **Sync workflows** GitHub Actions workflow from the [Actions](../../actions/workflows/sync.yaml) page and select **Run workflow**.

From the command line, run:

```sh
gh workflow run sync.yaml
```

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](./LICENSE) file for details.
