use std::ffi::OsString;

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

/// Workflow that has to be synced.
pub struct SyncWorkflow {
    /// Source file name retained in target repositories.
    pub file_name: OsString,
    /// Sync targets.
    pub sync: Vec<Repo>,
    /// Workflow contents without sync headers.
    pub contents: String,
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
