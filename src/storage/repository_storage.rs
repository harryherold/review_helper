use std::convert::From;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct RepositoryName(String);

impl From<&str> for RepositoryName {
    fn from(value: &str) -> Self {
        RepositoryName(value.to_string())
    }
}

impl From<&RepositoryName> for String {
    fn from(value: &RepositoryName) -> Self {
        value.0.clone()
    }
}

// #[derive(Debug, Default, PartialEq)]
// pub struct ReviewName(String);

#[derive(Debug, Default, PartialEq)]
pub struct RepositoryStore {
    pub path: PathBuf,
    pub name: RepositoryName,
    pub first_commit: String,
}

// pub struct ReviewStore {}

pub trait ReviewHelperStorage {
    fn load_repositories(&self) -> anyhow::Result<Vec<RepositoryStore>>;
    // fn store_repository(&self, repository_store: RepositoryStore) -> anyhow::Result<()>;
    // fn load_reviews(&self, repository_name: &RepositoryName) -> anyhow::Result<Vec<ReviewName>>;
    // fn load_review(&self, repository_name: &RepositoryName, review_name: &ReviewName) -> anyhow::Result<Option<ReviewStore>>;
    // fn store_review(&self, repository_name: &RepositoryName, review: ReviewStore) -> anyhow::Result<()>;
}
