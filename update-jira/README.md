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
| `extra-data` | Extra data merged into the request body; nest under `fields` (e.g. `{"fields": {"fixVersions": [...]}}`) | No | |

## Outputs

| Name | Description |
|------|-------------|
| `updated` | Number of issues updated |

## Example Usage

Bulk update multiple issues:

```yaml
- name: Move tickets to the next release
  uses: mongodb/apix-action/update-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-keys: CLOUDP-123,CLOUDP-456
    extra-data: '{"fields":{"fixVersions":[{"name":"next-atlascli-release"}]}}'
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
- Jira Server/Data Center (e.g. `jira.mongodb.org`) exposes no bulk-*edit* REST
  API — only bulk *create* (`/rest/api/2/issue/bulk`). The async bulk-edit
  endpoint (`/rest/api/3/bulk/issues/fields`) is Cloud-only, so per-issue PUT is
  the supported approach here. Fine for dozens of issues.
- `extra-data` must be a valid JSON object. It is merged into the request body
  (like `create-jira`), so nest fields under `fields` — e.g.
  `{"fields": {"fixVersions": [{"name": "..."}]}}`.
