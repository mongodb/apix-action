# App Token Action

A GitHub Action to get a GitHub App token and related information.

## Inputs

| Name | Description | Required |
|------|-------------|----------|
| `app-id` | GitHub App ID | Yes |
| `private-key` | GitHub App private key | Yes |

## Outputs

| Name | Description |
|------|-------------|
| `token` | GitHub installation access token |
| `installation-id` | GitHub App installation ID |
| `app-slug` | GitHub App slug |
| `user-name` | GitHub App User Name |
| `user-email` | GitHub App User Email |
| `user-id` | GitHub App User ID |
| `committer` | GitHub App Committer String |

## Example Usage

```yaml
- name: Get GitHub App Token
  id: app_token
  uses: your-org/apix-action/token@v1
  with:
    app-id: ${{ secrets.APP_ID }}
    private-key: ${{ secrets.PRIVATE_KEY }}

- name: Use the token
  run: |
    echo "Token: ${{ steps.app_token.outputs.token }}"
    echo "App User: ${{ steps.app_token.outputs.user-name }}"
    echo "Committer: ${{ steps.app_token.outputs.committer }}"
  
- name: Use with GitHub CLI
  env:
    GH_TOKEN: ${{ steps.app_token.outputs.token }}
  run: gh repo list
```

## Using for Git Operations

The action provides convenient outputs for git configuration:

```yaml
- name: Configure Git
  run: |
    git config --global user.name "${{ steps.app_token.outputs.user-name }}"
    git config --global user.email "${{ steps.app_token.outputs.user-email }}"
    
- name: Commit Changes
  run: |
    git add .
    git commit -m "Automated changes [skip ci]"
    git push
  env:
    GITHUB_TOKEN: ${{ steps.app_token.outputs.token }}
```