# Create Jira Issue Action

A GitHub Action to create a Jira issue with customizable fields.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `project-key` | Project Key | Yes | |
| `summary` | Summary of the issue | Yes | |
| `description` | Description of the issue | No | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |
| `issuetype` | Name of the issue type | No | |
| `assignee` | Assignee of the issue | No | |
| `status` | Status to transition the issue to after creation | No | |
| `labels` | Labels for the issue (comma separated) | No | |
| `components` | Components for the issue (comma separated) | No | |
| `extra-data` | Extra data to be merged in the final request | No | |

## Outputs

| Name | Description |
|------|-------------|
| `issue-key` | Key of the created Jira issue |

## Example Usage

```yaml
- name: Create Jira Issue
  id: create_jira
  uses: your-org/apix-action/create-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: PROJECT
    summary: "Bug: Something is broken"
    description: "Detailed description of the bug..."
    issuetype: Bug
    assignee: johndoe
    status: In Progress
    labels: bug,priority-high
    components: frontend,api
    extra-data: |
      {
        "customfield_10000": "Custom field value"
      }

- name: Use the issue key
  run: echo "Created issue ${{ steps.create_jira.outputs.issue-key }}"
```

## Custom Fields

Use the `extra-data` input to set custom fields and other advanced properties. The value should be valid JSON that will be merged with the main request body.

If `status` is set, the action creates the issue first and then transitions it. The value must match a target status available from the issue's initial Jira workflow state.

Example:

```yaml
extra-data: |
  {
    "customfield_10000": "Custom value",
    "customfield_10001": { "id": "10100" },
    "fields": {
      "customfield_10002": [{"id": "10101"}, {"id": "10102"}]
    }
  }
```
