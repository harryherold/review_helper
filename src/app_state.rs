use std::{collections::HashSet, convert::From, path::PathBuf, rc::Rc};

use slint::{ComponentHandle, Model, SharedString, VecModel};

// use crate::commit_proxy_model::CommitProxyModel;
// use crate::file_diff_proxy_models::FileDiffProxyModels;
// use crate::files_proxy_model::FilesProxyModel;
// use crate::notes_proxy_models::NotesProxyModels;
// use crate::project::Project;
use crate::{
    app_config, git_utils,
    id_model::IdModel,
    storage::{RepositoryName, RepositoryStore, ReviewHelperFileStorage, ReviewHelperStorage},
    ui,
};

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    RepositoryExists(String),
    GitCommandFailed(String),
    NoGitDirectory(String),
    StoreFailed(String),
}

pub struct ReviewHelperModel {
    storage: Box<dyn ReviewHelperStorage>,
    repositories_model: Rc<IdModel<ui::RepositoryUi>>,
    error_model: Rc<VecModel<ui::ErrorEntry>>,
    repository_paths: HashSet<PathBuf>,
    last_id: usize,
}

impl From<(usize, &RepositoryStore)> for ui::RepositoryUi {
    fn from((id, value): (usize, &RepositoryStore)) -> Self {
        ui::RepositoryUi {
            id: id as i32,
            first_commit: SharedString::from(&value.first_commit),
            name: String::from(&value.name).into(),
            path: value.path.as_os_str().to_str().unwrap_or_default().into(),
        }
    }
}

impl From<&ui::RepositoryUi> for RepositoryStore {
    fn from(value: &ui::RepositoryUi) -> Self {
        RepositoryStore {
            path: PathBuf::from(value.path.as_str()),
            name: RepositoryName::from(value.name.as_str()),
            first_commit: String::from(value.first_commit.as_str()),
        }
    }
}

fn path_to_str(path: &PathBuf) -> &str {
    path.to_str().unwrap_or_default()
}

impl ReviewHelperModel {
    fn new(storage: Box<dyn ReviewHelperStorage>) -> Self {
        let repository_stores = storage.load_repositories().expect("Error while loading repositories from config!");
        let model = IdModel::default();
        let mut paths = HashSet::new();

        repository_stores.iter().enumerate().for_each(|(id, item)| {
            paths.insert(item.path.clone());
            model.add(id + 1, ui::RepositoryUi::from((id + 1, item)));
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

        let ui_repository = ui::RepositoryUi {
            first_commit,
            id: self.last_id as i32,
            name: name.into(),
            path: path_str.into(),
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
    pub fn add_error(&self, result: ui::Result, text: SharedString) {
        self.error_model.push(ui::ErrorEntry { text, error_type: result });
    }
}

pub struct AppState {
    pub app_window: ui::AppWindow,
    pub app_config: app_config::AppConfig,
    pub model: ReviewHelperModel,
    // pub project: Rc<RefCell<Project>>,
    // pub file_diff_proxy_models: Rc<RefCell<FileDiffProxyModels>>,
    // pub commit_proxy_model: Rc<RefCell<CommitProxyModel>>,
    // pub notes_proxy_models: Rc<RefCell<NotesProxyModels>>,
    // pub files_proxy_model:  Rc<RefCell<FilesProxyModel>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
        app_data_path.push(std::env!("CARGO_CRATE_NAME")); // directory

        let app_config = match app_config::AppConfig::new(app_data_path.clone()) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e.to_string());
                app_config::AppConfig::default()
            }
        };

        let storage = ReviewHelperFileStorage::new(app_data_path);

        let model = ReviewHelperModel::new(Box::new(storage));

        let app_window = ui::AppWindow::new().expect("Error while creating app window!");

        app_window
            .global::<ui::RepositoriesUi>()
            .set_repositories(model.repositories_model.clone().into());

        app_window.global::<ui::Errors>().set_model(model.error_model.clone().into());

        AppState {
            app_window,
            app_config,
            model,
            // project: Rc::new(RefCell::new(Project::default())),
            // file_diff_proxy_models: Rc::new(RefCell::new(FileDiffProxyModels::default())),
            // commit_proxy_model: Rc::new(RefCell::new(CommitProxyModel::default())),
            // notes_proxy_models: Rc::new(RefCell::new(NotesProxyModels::default())),
            // files_proxy_model: Rc::new(RefCell::new(FilesProxyModel::default())),
        }
    }
}
