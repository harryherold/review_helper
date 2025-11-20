use std::convert::From;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryName(String);

impl RepositoryName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ReviewName(String);

impl From<&str> for ReviewName {
    fn from(value: &str) -> Self {
        ReviewName(value.to_string())
    }
}

impl From<&ReviewName> for String {
    fn from(value: &ReviewName) -> Self {
        value.0.clone()
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct RepositoryStore {
    pub path: PathBuf,
    pub name: RepositoryName,
    pub first_commit: String,
    pub base_branch: String,
}

// pub struct ReviewStore {}

pub trait ReviewHelperStorage {
    fn load_repositories(&self) -> anyhow::Result<Vec<RepositoryStore>>;
    fn save_repository(&self, repository_store: RepositoryStore) -> anyhow::Result<()>;
    fn load_review_names(&self, repository_name: &RepositoryName) -> anyhow::Result<Vec<ReviewName>>;
    // fn load_review(&self, repository_name: &RepositoryName, review_name: &ReviewName) -> anyhow::Result<Option<ReviewStore>>;
    // fn store_review(&self, repository_name: &RepositoryName, review: ReviewStore) -> anyhow::Result<()>;
}
