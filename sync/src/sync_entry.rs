use std::str::FromStr;

use crate::shared::Repo;

pub const PREFIX: &str = "# sync -> ";

pub struct SyncEntry {
    pub repo: Repo,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("invalid sync entry '{0}', expecting format: '# sync -> owner/name'")]
pub struct ParseSyncEntryError(String);

impl FromStr for SyncEntry {
    type Err = ParseSyncEntryError;

    /// Parse `# sync -> owner/name` into a target repository.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let create_err = || ParseSyncEntryError(s.to_string());

        let repo = s.strip_prefix(PREFIX).ok_or_else(create_err)?;
        let repo = repo.parse().map_err(|_| create_err())?;

        Ok(SyncEntry { repo })
    }
}

#[cfg(test)]
mod tests {
    use non_empty_string::NonEmptyString;

    use super::*;

    macro_rules! valid_entry {
        ($value: literal, $owner: literal, $name: literal) => {
            assert_eq!(
                Ok(Repo::new(
                    NonEmptyString::new($owner.to_string()).expect("non-empty owner"),
                    NonEmptyString::new($name.to_string()).expect("non-empty name")
                )),
                SyncEntry::from_str($value).map(|e| e.repo)
            )
        };
    }

    #[test]
    fn valid_entries() {
        valid_entry!(
            "# sync -> mongodb/atlas-local-lib",
            "mongodb",
            "atlas-local-lib"
        );
        valid_entry!(
            "# sync -> mongodb-js/atlas-local-lib-js",
            "mongodb-js",
            "atlas-local-lib-js"
        );
        valid_entry!(
            "# sync -> mongodb-labs/cobra2snooty",
            "mongodb-labs",
            "cobra2snooty"
        );
    }

    #[test]
    fn invalid_entries() {
        assert!(SyncEntry::from_str("mongodb/atlas-local-lib").is_err());
        assert!(SyncEntry::from_str("# sync -> ").is_err());
        assert!(SyncEntry::from_str("# sync -> /atlas-local-lib-js").is_err());
        assert!(SyncEntry::from_str("# sync -> mongodb-js/").is_err());
        assert!(SyncEntry::from_str("# sync -> ../atlas-local-lib").is_err());
        assert!(SyncEntry::from_str("# sync -> mongodb-js/../other").is_err());
        assert!(SyncEntry::from_str("# sync -> # sync -> mongodb/other").is_err());
    }
}
