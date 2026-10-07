# Link Jira Issues Action

A GitHub Action to create a link between two Jira issues.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `inward-issue` | Inward issue key (the issue on the inward side of the link type) | Yes | |
| `outward-issue` | Outward issue key (the issue on the outward side of the link type) | Yes | |
| `link-type` | Link type name, e.g. `related to`, `Blocks`, `Duplicate` | No | `related to` |
| `comment` | Optional comment to attach to the link | No | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Example Usage

```yaml
- name: Link release and notes tickets
  uses: mongodb/apix-action/link-issues@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    inward-issue: CLOUDP-123
    outward-issue: DOCSP-456
    link-type: "related to"
```
