use std::{collections::HashSet, ffi::OsString, str::FromStr};

use non_empty_string::NonEmptyString;
use nutype::nutype;

#[nutype(validate(not_empty), derive(Clone, Deref, FromStr))]
pub struct GithubToken(String);

impl GithubToken {
    /// Expose token only at authentication boundaries that require its raw value.
    pub(crate) fn expose_secret(&self) -> &str {
        self
    }
}

impl AsRef<[u8]> for GithubToken {
    /// Return token bytes for libraries that accept byte slices.
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Repo {
    pub owner: NonEmptyString,
    pub repository: NonEmptyString,
}

impl Repo {
    /// Create a repository identifier from validated owner and name components.
    pub fn new(owner: NonEmptyString, repository: NonEmptyString) -> Self {
        Self { owner, repository }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid repository '{0}', expecting format: 'owner/name'")]
pub struct ParseRepoError(String);

impl FromStr for Repo {
    type Err = ParseRepoError;

    /// Parse `owner/name` into a repository identifier.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let create_err = || ParseRepoError(s.to_string());

        let (owner, name) = s.split_once('/').ok_or_else(create_err)?;

        if !is_safe_path_component(owner) || !is_safe_path_component(name) {
            return Err(create_err());
        }

        // Convert to non-empty strings
        let owner = owner.try_into().map_err(|_| create_err())?;
        let name = name.try_into().map_err(|_| create_err())?;

        Ok(Repo::new(owner, name))
    }
}

/// Reject path traversal and characters that could produce unsafe checkout paths.
pub fn is_safe_path_component(value: &str) -> bool {
    !matches!(value, "" | "." | "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Workflow that has to be synced.
pub struct SyncWorkflow {
    /// Source file name retained in target repositories.
    pub file_name: OsString,
    /// Sync targets.
    pub sync: Vec<Repo>,
    /// Workflow contents without sync headers.
    pub contents: String,
}

/// Workflow file names the source writes to one repository.
pub fn expected_workflows<'a>(
    workflows: &'a [SyncWorkflow],
    repository: &Repo,
) -> HashSet<&'a str> {
    workflows
        .iter()
        .filter(|workflow| workflow.sync.contains(repository))
        .filter_map(|workflow| workflow.file_name.to_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use redacted::FullyRedacted;

    use super::GithubToken;

    #[test]
    fn github_token_formatting_is_redacted() {
        let token = FullyRedacted::new(
            "secret-token"
                .parse::<GithubToken>()
                .expect("non-empty token"),
        );

        assert!(!format!("{token:?}").contains("secret-token"));
        assert!(!format!("{token}").contains("secret-token"));
    }
}
