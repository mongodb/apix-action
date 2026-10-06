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

A reusable workflow (`_*.yaml`) is not copied into targets — the synced caller reaches it by
permalink. GitHub resolves `./` inside it against the caller's workspace, so a reusable
workflow cannot use `./` at all and must pin this repository's own actions by permalink, as
`_dependabot-jira.yaml` does for `commit-changes`.

## Removing a workflow from a target

Remove the `# sync -> owner/repository` header to stop syncing to that target, or delete the workflow entirely to retire it everywhere. The next sync run deletes the copy from each affected repository and lists it under `Actions removed` in the generated PR.

Each run sweeps every repository the app is installed on, not just current targets, so a repository dropped from every header is still cleaned up. Repositories are inspected over the API and only cloned when they actually need a change.

Only files carrying the generated `# synced from apix-actions/...` marker are removed, so workflows a target repository owns itself are never touched.

The owner matrix is built by listing the app's installations, so an owner is swept even once all of its workflows are retired.

A run that finds no syncable workflows at all refuses to sync, so a misconfigured workflow directory cannot be mistaken for "everything retired".

# Dependabot post-update task

`_dependabot-jira.yaml` runs a repository's `.github/dependabot-post-update.sh` (when present)
before approving or merging a Dependabot PR. The script is repository-owned, so sync never
writes or removes it; repositories without the file skip the step.

The caller triggers on `opened` and `synchronize`, so a Dependabot rebase or recreate
regenerates the derived files too. The job accepts `dependabot[bot]` and this app's own bot:
the run the post-update commit triggers is the one that approves, because only by then has the
PR head caught up with the commit that was just pushed. The script itself still runs only for
`dependabot[bot]`, so that follow-up run does not regenerate a second time, and a run that
pushed a commit skips approving so that only the settled head is approved. Two further guards
make the re-runs safe: the ticket, comment, and `auto_close_jira` label steps are skipped once
the PR already carries that label, so no duplicate tickets are filed; and every commit in the
PR must be authored by `dependabot[bot]` or by this app's own bot and signed by GitHub, so a
write-access user cannot smuggle a commit onto a Dependabot branch and have it auto-merged and
executed.

The script only edits files in the checked-out PR branch — no git commands. The workflow stages
the result, writes the commit message, and creates the commit through the GitHub API
(`createCommitOnBranch`), so GitHub signs it and it shows as Verified.

Merging waits for the required status checks and then merges directly, rather than arming
auto-merge. Auto-merge also waits on the code-owner review, which the app bypasses through the
ruleset's bypass list but auto-merge does not honour, so a repository with
`require_code_owner_review` would never merge. The app performs the merge itself so its bypass
applies, and the checks are still enforced because the workflow waits for them first.

Two repository variables (Settings → Variables) configure it:

- `DEPENDABOT_COMMIT_MESSAGE` — commit headline; defaults to `chore: regenerate derived files`.
- `DEPENDABOT_AUTO_APPROVE` — set to `true` to approve and merge once the required checks pass;
  defaults to off, leaving the PR for human review.
