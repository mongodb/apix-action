# Workflow Sync

Add `# sync -> owner/repository` to workflow under `.github/workflows/` to add target.

Merge workflow changes to `main`, then manually run the `Sync workflows` GitHub Action to propagate changes and open generated PRs.

## Referencing reusable workflows and actions

Inside a synced workflow, reference anything from this repository by relative path:

```yaml
uses: ./.github/workflows/_close-jira.yaml
```

Do not pin `mongodb/apix-action/...@<sha>` by hand. Sync rewrites each `./` reference to a
permalink when it writes the file into a target, pinning every reference in the run to the
commit being synced. So the permalink always matches the content being shipped, and
dependabot has no self-reference to bump here.

Because the pin is the synced commit, any run where `main` has moved rewrites every synced
caller and opens a pull request per target, even when the reusable workflow itself did not
change.

Sync must be dispatched from `main`; the workflow fails otherwise. A commit on a branch may
never reach `main`, which would leave targets pinned to a permalink that dies with the
branch.

A `./` reference that does not exist fails the run rather than shipping a broken `uses:`,
since the unrewritten form would resolve against the target repository.

## Removing a workflow from a target

Remove the `# sync -> owner/repository` header to stop syncing to that target, or delete the workflow entirely to retire it everywhere. The next sync run deletes the copy from each affected repository and lists it under `Actions removed` in the generated PR.

Each run sweeps every repository the app is installed on, not just current targets, so a repository dropped from every header is still cleaned up. Repositories are inspected over the API and only cloned when they actually need a change.

Only files carrying the generated `# synced from apix-actions/...` marker are removed, so workflows a target repository owns itself are never touched.

The owner matrix is built by listing the app's installations, so an owner is swept even once all of its workflows are retired.

A run that finds no syncable workflows at all refuses to sync, so a misconfigured workflow directory cannot be mistaken for "everything retired".
