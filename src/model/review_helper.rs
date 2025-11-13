use std::{collections::HashSet, convert::From, path::PathBuf, rc::Rc};

use slint::{Model, SharedString, VecModel};

use crate::{
    git_utils,
    model::IdModel,
    storage::{RepositoryName, RepositoryStore, ReviewHelperStorage},
    ui,
};

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    RepositoryExists(String),
    GitCommandFailed(String),
    NoGitDirectory(String),
    StoreFailed(String),
}

// pub struct RepositoryModel {
//     pub commit_proxy_model: CommitProxyModel,
// }

pub struct ReviewHelperModel {
    storage: Box<dyn ReviewHelperStorage>,
    pub repositories_model: Rc<IdModel<ui::SlintRepository>>,
    pub error_model: Rc<VecModel<ui::SlintErrorEntry>>,
    // pub repository_models: BTreeMap<usize, RepositoryModel>,
    repository_paths: HashSet<PathBuf>,
    last_id: usize,
}

impl From<(usize, &RepositoryStore)> for ui::SlintRepository {
    fn from((id, value): (usize, &RepositoryStore)) -> Self {
        ui::SlintRepository {
            id: id as i32,
            first_commit: SharedString::from(&value.first_commit),
            name: String::from(&value.name).into(),
            path: value.path.as_os_str().to_str().unwrap_or_default().into(),
            base_branch: SharedString::from(&value.base_branch),
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

impl ReviewHelperModel {
    pub fn new(storage: Box<dyn ReviewHelperStorage>) -> Self {
        let repository_stores = storage.load_repositories().expect("Error while loading repositories from config!");
        let model = IdModel::default();
        let mut paths = HashSet::new();

        repository_stores.iter().enumerate().for_each(|(id, item)| {
            paths.insert(item.path.clone());
            model.add(id + 1, ui::SlintRepository::from((id + 1, item)));
        });

        let last_id = model.row_count() + 1;
        Self {
            storage,
            repositories_model: Rc::new(model),
            error_model: Rc::new(VecModel::default()),
            repository_paths: paths,
            last_id: last_id,
        }
    }
    pub fn add_repository(&mut self, path: PathBuf) -> Result<(), ReviewHelperError> {
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

        let ui_repository = ui::SlintRepository {
            first_commit,
            id: self.last_id as i32,
            name: name.into(),
            path: path_str.into(),
            base_branch: SharedString::from("main"),
        };

        let repository_store = RepositoryStore::from(&ui_repository);
        self.storage
            .save_repository(repository_store)
            .map_err(|e| ReviewHelperError::StoreFailed(e.to_string()))?;

        self.repository_paths.insert(path.clone());
        self.repositories_model.add(self.last_id, ui_repository);
        self.last_id += 1;

        Ok(())
    }
    pub fn add_error(&self, result: ui::SlintResult, text: SharedString) {
        self.error_model.push(ui::SlintErrorEntry { text, error_type: result });
    }
}
