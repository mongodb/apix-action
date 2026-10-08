# Create Jira Version Action

A GitHub Action to create a Jira project version (fix version). It is a plain
create: it fails if a version with the given name already exists. To create only
when missing, check with [`get-jira-version`](../get-jira-version/README.md)
first (see the example below).

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `project-key` | Project key that owns the version | Yes | |
| `name` | Version name to create | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `version-id` | Id of the newly created version |

## Example Usage

```yaml
- name: Create next fix version
  id: create_version
  uses: mongodb/apix-action/create-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release

- name: Use the version id
  run: echo "version id ${{ steps.create_version.outputs.version-id }}"
```

## Create only when missing

```yaml
- name: Find the rolling next fix version
  id: find_version
  uses: mongodb/apix-action/get-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release

- name: Create the rolling next fix version
  if: steps.find_version.outputs.found != 'true'
  uses: mongodb/apix-action/create-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release
```

## Errors

A non-2xx response from Jira is reported with its HTTP status and response body,
so failures explain themselves rather than surfacing as a bare `curl` exit code.
