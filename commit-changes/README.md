# Commit changes

Stages the working tree and commits it through the GitHub API
(`createCommitOnBranch`), so GitHub signs the commit and it shows as Verified. No git
identity or signing setup is needed in the caller.

The repository must already be checked out on a branch (not a detached HEAD) with a token
that has `contents: write`. When there is nothing to commit, the action succeeds with an
empty `oid` output. `paths` scopes the commit to specific paths and defaults to the whole
working tree.

```yaml
- uses: actions/checkout@<sha> # v7
  with:
    ref: ${{ github.head_ref }}
    token: ${{ steps.app-token.outputs.token }}
- uses: mongodb/apix-action/commit-changes@<sha>
  with:
    token: ${{ steps.app-token.outputs.token }}
    message: chore: regenerate derived files
    body: Regenerated after ${{ github.event.pull_request.title }}
    paths: dist generated
```
