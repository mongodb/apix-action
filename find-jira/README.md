# Find Jira Issue Action

A GitHub Action to find Jira issues using JQL (Jira Query Language).

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `jql` | JQL query to run | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `found` | Returns 'true' if issue is found otherwise 'false' |
| `issue-key` | Key of the issue found (if any) |

## Example Usage

```yaml
- name: Find Jira Issue
  id: find_jira
  uses: your-org/apix-action/find-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    jql: "project = PROJECT AND summary ~ 'Bug report'"
    api-base: "https://jira.example.org"

- name: Use the result
  if: steps.find_jira.outputs.found == 'true'
  run: echo "Found issue ${{ steps.find_jira.outputs.issue-key }}"

- name: No issue found
  if: steps.find_jira.outputs.found == 'false'
  run: echo "No matching issue found"
```

## JQL Examples

### Find by project and type
```
project = PROJECT AND issuetype = Bug
```

### Find by status and assignee
```
project = PROJECT AND status = "In Progress" AND assignee = currentUser()
```

### Find by label and creation date
```
project = PROJECT AND labels = "critical" AND created >= -7d
```