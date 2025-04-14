# Transition Jira Issue Action

A GitHub Action to transition Jira issues between states.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `issue-key` | Issue Key | Yes | |
| `transition-id` | ID of the transition | Yes | |
| `resolution` | Resolution of the transition | No | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Example Usage

```yaml
- name: Transition Jira Issue
  uses: your-org/apix-action/transition-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-key: PROJECT-123
    transition-id: '31' # ID for the "In Progress" transition
    api-base: "https://jira.example.org"
```

## Example with Resolution

```yaml
- name: Resolve Jira Issue
  uses: your-org/apix-action/transition-jira@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    issue-key: PROJECT-123
    transition-id: '5' # ID for the "Resolve Issue" transition
    resolution: "Done"
    api-base: "https://jira.example.org"
```

## Finding Transition IDs

To find the available transition IDs for an issue:

1. Make a GET request to the Jira API:
   ```
   curl -X GET \
     --url "https://jira.example.org/rest/api/2/issue/PROJECT-123/transitions" \
     --header "Authorization: Bearer YOUR_TOKEN" \
     --header "Accept: application/json"
   ```

2. The response will include transition IDs and names:
   ```json
   {
     "transitions": [
       {
         "id": "31",
         "name": "In Progress"
       },
       {
         "id": "5",
         "name": "Resolve Issue"
       }
     ]
   }
   ```