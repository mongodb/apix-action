# Update Jira Version Action

A GitHub Action to update a Jira project version (fix version): rename it, set its
release date, and/or mark it released. The version is located by its current
`name`.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `project-key` | Project key that owns the version | Yes | |
| `name` | Current version name used to locate the version | Yes | |
| `new-name` | New version name (rename) | No | |
| `release-date` | Release date in `YYYY-MM-DD` | No | |
| `released` | `true` or `false` | No | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `version-id` | Id of the updated version |

## Example Usage

Rename a version and set its release date:

```yaml
- name: Rename and schedule
  uses: mongodb/apix-action/update-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release
    new-name: atlascli-1.60.0
    release-date: "2026-10-07"
```

Mark a version as released:

```yaml
- name: Mark released
  uses: mongodb/apix-action/update-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: atlascli-1.60.0
    released: "true"
    release-date: "2026-10-07"
```
