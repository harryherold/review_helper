use crate::commit_proxy_model::CommitProxyModel;
use crate::file_diff_proxy_models::FileDiffModelContext;
use crate::notes_proxy_models::NotesProxyModels;
use crate::project::Project;
use crate::{app_config, ui};
use std::cell::RefCell;
use std::rc::Rc;

pub struct AppState {
    pub app_window: ui::AppWindow,
    pub app_config: Rc<RefCell<app_config::AppConfig>>,
    pub project: Rc<RefCell<Project>>,
    pub file_diff_proxy_models: Rc<RefCell<FileDiffModelContext>>,
    pub commit_proxy_model: Rc<RefCell<CommitProxyModel>>,
    pub notes_proxy_models: Rc<RefCell<NotesProxyModels>>,
}

impl AppState {
    pub fn new() -> Self {
        let app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
        let app_config = match app_config::AppConfig::new(app_data_path) {
            Ok(config) => Rc::new(RefCell::new(config)),
            Err(e) => {
                eprintln!("{}", e.to_string());
                Rc::new(RefCell::new(app_config::AppConfig::default()))
            }
        };

        AppState {
            app_window: ui::AppWindow::new().unwrap(),
            app_config,
            project: Rc::new(RefCell::new(Project::default())),
            file_diff_proxy_models: Rc::new(RefCell::new(FileDiffModelContext::default())),
            commit_proxy_model: Rc::new(RefCell::new(CommitProxyModel::default())),
            notes_proxy_models: Rc::new(RefCell::new(NotesProxyModels::default())),
        }
    }
}
