use std::path::PathBuf;

#[derive(Debug, Default, PartialEq)]
pub struct RepositoryStore {
    pub path: PathBuf,
    pub name: String,
    pub first_commit: String,
}

#[derive(Debug, Default)]
pub struct ReviewHelperStore {
    pub repositories: Vec<RepositoryStore>,
}

pub trait ReviewHelperStorage {
    fn load(&self) -> anyhow::Result<ReviewHelperStore>;
}
