# Create Jira Version Action

A GitHub Action to create a Jira project version (fix version). Idempotent: if a
version with the given name already exists, its id is returned and no new version
is created.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `project-key` | Project key that owns the version | Yes | |
| `name` | Version name to create | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `version-id` | Id of the existing or newly created version |

## Example Usage

```yaml
- name: Create next fix version
  id: create_version
  uses: mongodb/apix-action/create-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release

- name: Use the version id
  run: echo "version id ${{ steps.create_version.outputs.version-id }}"
```
