use std::path::PathBuf;

pub mod repository_file_storage;
pub mod repository_storage;

pub use repository_file_storage::ReviewHelperFileStorage;
pub use repository_storage::RepositoryName;
pub use repository_storage::RepositoryStore;
pub use repository_storage::ReviewHelperStorage;
pub use repository_storage::StorageResult;

pub fn create_storage(path: PathBuf) -> Box<dyn ReviewHelperStorage> {
    Box::new(ReviewHelperFileStorage::new(path))
}
