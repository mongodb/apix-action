# Update Jira Version Action

A GitHub Action to update a Jira project version (fix version) by id: rename it,
set its release date, and/or mark it released. It takes the version `version-id`
directly — use [`get-jira-version`](../get-jira-version/README.md) to resolve a
name to an id first.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `version-id` | Id of the version to update | Yes | |
| `new-name` | New version name (rename) | No | |
| `release-date` | Release date in `YYYY-MM-DD` | No | |
| `released` | `true` or `false` | No | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `version-id` | Id of the updated version |

## Example Usage

Resolve a version by name, then rename it and set its release date:

```yaml
- name: Find the version
  id: find_version
  uses: mongodb/apix-action/get-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release

- name: Rename and schedule
  uses: mongodb/apix-action/update-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    version-id: ${{ steps.find_version.outputs.version-id }}
    new-name: atlascli-1.60.0
    release-date: "2026-10-07"
```

Mark a version as released:

```yaml
- name: Mark released
  uses: mongodb/apix-action/update-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    version-id: ${{ steps.find_version.outputs.version-id }}
    released: "true"
    release-date: "2026-10-07"
```

## Errors

A non-2xx response from Jira is reported with its HTTP status and response body,
so failures explain themselves rather than surfacing as a bare `curl` exit code.
