# Find Jira Issue Action

A GitHub Action to find Jira issues using JQL (Jira Query Language).

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `jql` | JQL query to run | Yes | |
| `fields` | Comma-separated list of fields to return for each issue | No | |
| `max-results` | Maximum number of issues to return | No | `50` |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `found` | Returns 'true' if any issue is found otherwise 'false' |
| `issue-key` | Key of the first issue found (if any) |
| `issues` | JSON array of all issues found (including the requested `fields`) |

## Example Usage

```yaml
- name: Find Jira Issue
  id: find_jira
  uses: mongodb/apix-action/find-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    jql: "project = PROJECT AND summary ~ 'Bug report'"
    api-base: "https://jira.example.org"

- name: Use the result
  if: steps.find_jira.outputs.found == 'true'
  run: echo "Found issue ${{ steps.find_jira.outputs.issue-key }}"
```

## Example Usage with fields

```yaml
- name: Find issues with release notes
  id: find_notes
  uses: mongodb/apix-action/find-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    jql: 'project = CLOUDP AND fixVersion = "atlascli-1.60.0" AND "Release Notes" IS NOT EMPTY'
    fields: "summary,Release Notes"
    max-results: "1000"

- name: Print the extracted notes
  run: |
    echo '${{ steps.find_notes.outputs.issues }}' | jq -r '.[].fields["Release Notes"]'
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
