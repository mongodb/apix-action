# Workflow Sync

Add `# sync -> owner/repository` to workflow under `.github/workflows/` to add target. Remove header to stop syncing target.

Run `just sync-with-gh-token` locally to propagate changes and open generated PRs. GitHub Actions sync trigger is WIP.
