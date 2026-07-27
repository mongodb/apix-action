use non_empty_string::NonEmptyString;

#[derive(Debug, PartialEq, Eq)]
pub struct Repo {
    pub owner: NonEmptyString,
    pub repository: NonEmptyString,
}

impl Repo {
    pub fn new(owner: NonEmptyString, repository: NonEmptyString) -> Self {
        Self { owner, repository }
    }
}
