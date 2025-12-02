use std::{
    collections::{HashMap, HashSet},
    convert::From,
    path::PathBuf,
    rc::Rc,
};

use slint::{SharedString, VecModel};

use crate::{
    git_utils,
    storage::{RepositoryName, RepositoryStore, ReviewHelperStorage},
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
    // pub review_names_model: Rc<VecModel<SharedString>>,
}

impl Repository {
    pub fn new(name: &RepositoryName) -> Self {
        Self {
            name: name.clone(),
            // review_names_model: Rc::new(VecModel::default()),
        }
    }
    // pub fn update(&self, names: Vec<ReviewName>) {
    //     self.review_names_model.clear();
    //     for name in names {
    //         self.review_names_model.push(SharedString::from(name.as_str()));
    //     }
    // }
}

pub struct ReviewHelper {
    pub storage: Rc<dyn ReviewHelperStorage>,
    // pub repositories_model: Rc<IdModel<ui::SlintRepository>>,
    // pub error_model: Rc<VecModel<ui::SlintErrorEntry>>,

    // TODO @repository_stores can be dropped using a load function that returns these stores
    pub repository_stores: Vec<RepositoryStore>,
    pub repositories: HashMap<RepositoryName, Repository>,
    repository_paths: HashSet<PathBuf>,
    // last_id: usize,
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

fn path_to_str(path: &PathBuf) -> &str {
    path.to_str().unwrap_or_default()
}

impl ReviewHelper {
    pub fn new(storage: Rc<dyn ReviewHelperStorage>) -> Self {
        let repository_stores = storage.load_repositories().expect("Error while loading repositories from config!");
        let mut paths = HashSet::new();
        let mut repositories = HashMap::new();

        repository_stores.iter().for_each(|item| {
            paths.insert(item.path.clone());
            repositories.insert(item.name.clone(), Repository::new(&item.name));
        });

        Self {
            storage,
            repository_stores: repository_stores,
            repository_paths: paths,
            repositories,
        }
    }
    pub fn add_repository(&mut self, path: PathBuf) -> Result<RepositoryStore, ReviewHelperError> {
        let path_str = path_to_str(&path);

        if !git_utils::is_git_repo(&path) {
            return Err(ReviewHelperError::NoGitDirectory(path_str.to_string()));
        }

        if self.repository_paths.contains(&path) {
            return Err(ReviewHelperError::RepositoryExists(path_str.to_string()));
        }

        let name = path.file_name().unwrap_or_default().to_str().unwrap_or_default();
        let first_commit = git_utils::first_commit(&path)
            .map_err(|e| ReviewHelperError::GitCommandFailed(e.to_string()))?
            .into();

        let repository_name = RepositoryName::from(name);
        let repository = Repository::new(&repository_name);

        self.repository_paths.insert(path.clone());
        self.repositories.insert(repository_name.clone(), repository);

        let repository_store = RepositoryStore {
            base_branch: "main".to_string(),
            path: path,
            first_commit,
            name: repository_name,
        };

        self.storage
            .save_repository(&repository_store)
            .map_err(|e| ReviewHelperError::StoreFailed(e.to_string()))?;

        self.repository_stores.push(repository_store.clone());

        Ok(repository_store)
    }
}
