use std::{
    collections::{HashMap, HashSet},
    convert::From,
    path::PathBuf,
    rc::Rc,
};

use slint::{SharedString, VecModel};

use crate::{
    storage::{RepositoryName, RepositoryStore, repository_storage::ReviewName},
    ui,
};

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    RepositoryExists(String),
    GitCommandFailed(String),
    NoGitDirectory(String),
    StoreFailed(String),
    // ModelItemNotExists,
    // LoadReviewNamesFailed(String),
}

#[derive(Default, Clone)]
pub struct Repository {
    pub name: RepositoryName,
    pub store: RepositoryStore,
    pub review_names: Vec<ReviewName>,
}

impl Repository {
    pub fn new(name: &RepositoryName, store: RepositoryStore) -> Self {
        Self {
            name: name.clone(),
            store,
            review_names: Vec::new(),
        }
    }
    pub fn set_review_names(&mut self, names: Vec<ReviewName>) {
        self.review_names = names;
    }
}

#[derive(Default)]
pub struct ReviewHelperCache {
    pub repositories: HashMap<RepositoryName, Repository>,
    repository_paths: HashSet<PathBuf>,
}

impl From<(usize, &RepositoryStore)> for ui::SlintRepository {
    fn from((id, value): (usize, &RepositoryStore)) -> Self {
        ui::SlintRepository {
            id: id as i32,
            first_commit: SharedString::from(&value.first_commit),
            name: String::from(&value.name).into(),
            path: value.path.as_os_str().to_str().unwrap_or_default().into(),
            base_branch: SharedString::from(&value.base_branch),
            review_names: Rc::new(VecModel::default()).into(),
        }
    }
}

impl From<&ui::SlintRepository> for RepositoryStore {
    fn from(value: &ui::SlintRepository) -> Self {
        RepositoryStore {
            first_commit: String::from(value.first_commit.as_str()),
            name: RepositoryName::from(value.name.as_str()),
            path: PathBuf::from(value.path.as_str()),
            base_branch: String::from(value.base_branch.as_str()),
        }
    }
}

impl ReviewHelperCache {
    pub fn set_repositories(&mut self, repository_stores: &Vec<RepositoryStore>) {
        self.repositories.clear();
        self.repository_paths.clear();

        repository_stores.iter().for_each(|item| {
            self.repository_paths.insert(item.path.clone());
            self.repositories.insert(item.name.clone(), Repository::new(&item.name, item.clone()));
        });
    }
    pub fn add_repository(&mut self, store: RepositoryStore) {
        self.repository_paths.insert(store.path.clone());

        let repository_name = store.name.clone();

        let repository = Repository::new(&repository_name, store);

        self.repositories.insert(repository_name, repository);
    }
    pub fn contains_repository_path(&self, path: &PathBuf) -> bool {
        self.repository_paths.contains(path)
    }
    pub fn get_mut_repository(&mut self, name: &RepositoryName) -> Option<&mut Repository> {
        self.repositories.get_mut(name)
    }
}
