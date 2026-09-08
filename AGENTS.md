# Workflow Sync

Add `# sync -> owner/repository` to workflow under `.github/workflows/` to add target.

Merge workflow changes to `main`, then manually run the `Sync workflows` GitHub Action to propagate changes and open generated PRs.

## Removing a workflow from a target

Remove the `# sync -> owner/repository` header to stop syncing to that target, or delete the workflow entirely to retire it everywhere. The next sync run deletes the copy from each affected repository and lists it under `Actions removed` in the generated PR.

Each run sweeps every repository the app is installed on, not just current targets, so a repository dropped from every header is still cleaned up. Repositories are inspected over the API and only cloned when they actually need a change.

Only files carrying the generated `# synced from apix-actions/...` marker are removed, so workflows a target repository owns itself are never touched.

The owner matrix is still built from `# sync ->` headers, so an owner with no remaining targets is not swept. Keep at least one header per owner until its workflows are cleaned up.
