# Update Jira Issue Action

A GitHub Action to update fields on one or more Jira issues. Accepts a single
issue key or many, so it doubles as a bulk editor. Mirrors the field inputs of
[`create-jira`](../create-jira/README.md).

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `issue-keys` | Issue key(s) to update. Comma or newline separated (one or many) | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |
| `summary` | Summary of the issue | No | |
| `description` | Description of the issue | No | |
| `assignee` | Assignee of the issue | No | |
| `labels` | Labels for the issue. Comma separated | No | |
| `components` | Components for the issue. Comma separated | No | |
| `extra-data` | Extra data to be merged in the final request (e.g. `fixVersions`) | No | |

## Outputs

| Name | Description |
|------|-------------|
| `updated` | Number of issues updated |

## Example Usage

Move unresolved tickets to the next release (bulk):

```yaml
- name: Move unresolved tickets to the next release
  uses: mongodb/apix-action/update-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-keys: ${{ steps.unresolved.outputs.issue-keys }}
    extra-data: '{"fixVersions":[{"name":"next-atlascli-release"}]}'
```

Update a single issue's fields:

```yaml
- name: Reassign and label
  uses: mongodb/apix-action/update-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-keys: CLOUDP-123
    assignee: opsmanager.serviceaccount
    labels: "atlascli,release"
```

## Notes

- Issues are updated with `PUT /rest/api/2/issue/{key}` one at a time, so the
  step fails fast on the first error. Re-running is safe (idempotent).
- `extra-data` must be a valid JSON object; it is merged into the request's
  `fields` (last write wins).
