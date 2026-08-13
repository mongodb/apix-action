# Workflow Sync

Add `# sync -> owner/repository` to workflow under `.github/workflows/` to add target. Remove header to stop syncing target.

Merge workflow changes to `main`, then manually run the `Sync workflows` GitHub Action to propagate changes and open generated PRs.
