# Update Jira Issue Action

A GitHub Action to update fields on one or more Jira issues. Accepts a single
issue key or many, so it doubles as a bulk editor.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `issue-keys` | Issue key(s) to update. Comma or newline separated (one or many) | Yes | |
| `fields` | JSON object of fields to set, merged into the request's `fields` | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `updated` | Number of issues updated |

## Example Usage

Update a single issue:

```yaml
- name: Set fix version
  uses: mongodb/apix-action/update-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-keys: CLOUDP-123
    fields: '{"fixVersions":[{"name":"next-atlascli-release"}]}'
```

Bulk update (one or many keys):

```yaml
- name: Move unresolved tickets to the next release
  uses: mongodb/apix-action/update-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-keys: ${{ steps.unresolved.outputs.issue-keys }}
    fields: '{"fixVersions":[{"name":"next-atlascli-release"}]}'
```

## Notes

- Issues are updated with `PUT /rest/api/2/issue/{key}` one at a time, so the
  step fails fast on the first error. Re-running is safe (idempotent).
- `fields` must be a valid JSON object; it is merged under the request's
  `fields` key.
