# Verify Changed Files Action

A GitHub Action to check if specified files have changed between git refs and output the results.

## Inputs

| Name | Description | Required | Default |
|------|-------------|----------|---------|
| `files` | Files or patterns to check for changes | No | `.` (entire repository) |
| `base-ref` | Base git ref to compare changes against | No | `HEAD^` |
| `head-ref` | Head git ref to compare changes with | No | `HEAD` |
| `separator` | Separator for the output list of changed files | No | ` ` (space) |

## Outputs

| Name | Description |
|------|-------------|
| `changed_files` | List of all changed files |
| `files_changed` | Boolean indicating if any files have changed (true/false) |
| `all_files_changed` | Boolean indicating if all specified files have changed (true/false) |

## Example Usage

```yaml
name: Check File Changes

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  check_changes:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Fetch all history for comparing changes
          
      - name: Verify Changed Files
        id: verify_changed
        uses: your-org/verify-changed-files@v1
        with:
          files: |
            src/**/*.ts
            package.json
            
      - name: Run if files changed
        if: steps.verify_changed.outputs.files_changed == 'true'
        run: |
          echo "The following files have changed: ${{ steps.verify_changed.outputs.changed_files }}"
          # Run your build, test or deploy commands here
```

## Example with Default Files (Entire Repository)
```yaml
- name: Check Any Changes
  id: verify_any_changes
  uses: your-org/verify-changed-files@v1
  # No files input means check entire repository

- name: Run if any file changed
  if: steps.verify_any_changes.outputs.files_changed == 'true'
  run: echo "Something changed in the repository!"
```

