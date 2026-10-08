# Get Jira Version Action

A GitHub Action to look up a Jira project version (fix version) by name. It is a
pure lookup: it never creates or changes anything. Use it when you need to know
whether a version exists (and its id) before deciding what to do.

It makes a single request to the project's versions endpoint, which returns every
version in one array (that endpoint is not paginated).

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `token` | Token to connect to Jira | Yes | |
| `project-key` | Project key that owns the version | Yes | |
| `name` | Version name to look up | Yes | |
| `api-base` | Base URL for the Jira API | No | `https://jira.mongodb.org` |

## Outputs

| Name | Description |
|------|-------------|
| `version-id` | Id of the version, empty when no version with that name exists |
| `found` | `'true'` when a version with the given name exists, otherwise `'false'` |

## Example Usage

```yaml
- name: Look up the next fix version
  id: get_version
  uses: mongodb/apix-action/get-jira-version@v1
  with:
    token: ${{ secrets.JIRA_API_TOKEN }}
    project-key: CLOUDP
    name: next-atlascli-release

- name: Use the result
  if: steps.get_version.outputs.found == 'true'
  run: echo "version id ${{ steps.get_version.outputs.version-id }}"
```

## Errors

A non-2xx response from Jira is reported with its HTTP status on stderr, so
failures explain themselves rather than surfacing as a bare `curl` exit code.
