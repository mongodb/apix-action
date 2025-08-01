# Comment Jira Issue Action

A GitHub Action to comment on Jira issues.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `issue-key` | Issue Key | Yes | |
| `comment` | Comment to post | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Example Usage

```yaml
- name: Comment on Jira Issue
  uses: your-org/apix-action/comment-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-key: PROJECT-123
    comment: "This issue has been addressed in the latest release."
    api-base: "https://jira.example.org"
```

## Example with Detailed Comment

```yaml
- name: Add Detailed Comment to Jira Issue
  uses: your-org/apix-action/comment-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-key: PROJECT-123
    comment: |
      **Build Status**: ✅ Successful
      **Commit**: abc123def456
      **Environment**: Production
      **Deployed**: 2024-01-15 10:30 UTC
    api-base: "https://jira.example.org"
```
