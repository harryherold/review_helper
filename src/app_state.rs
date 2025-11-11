use std::fs;

use slint::ComponentHandle;

// use crate::commit_proxy_model::CommitProxyModel;
// use crate::file_diff_proxy_models::FileDiffProxyModels;
// use crate::files_proxy_model::FilesProxyModel;
// use crate::notes_proxy_models::NotesProxyModels;
// use crate::project::Project;
use crate::{review_helper::ReviewHelperModel, review_helper_settings, storage::ReviewHelperFileStorage, ui};

pub struct AppState {
    pub app_window: ui::AppWindow,
    pub review_helper_settings: review_helper_settings::ReviewHelperSettings,
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
        app_data_path.push(std::env!("CARGO_CRATE_NAME"));
        if !app_data_path.exists() {
            let result = fs::create_dir(&app_data_path);
            assert!(result.is_ok());
        }

        let review_helper_settings = match review_helper_settings::ReviewHelperSettings::new(app_data_path.clone()) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e.to_string());
                review_helper_settings::ReviewHelperSettings::default()
            }
        };

        let storage = ReviewHelperFileStorage::new(app_data_path);

        let model = ReviewHelperModel::new(Box::new(storage));

        let app_window = ui::AppWindow::new().expect("Error while creating app window!");

        app_window
            .global::<ui::SlintReviewHelper>()
            .set_repositories(model.repositories_model.clone().into());

        app_window.global::<ui::SlintErrors>().set_model(model.error_model.clone().into());

        AppState {
            app_window,
            review_helper_settings,
            model,
            // project: Rc::new(RefCell::new(Project::default())),
            // file_diff_proxy_models: Rc::new(RefCell::new(FileDiffProxyModels::default())),
            // commit_proxy_model: Rc::new(RefCell::new(CommitProxyModel::default())),
            // notes_proxy_models: Rc::new(RefCell::new(NotesProxyModels::default())),
            // files_proxy_model: Rc::new(RefCell::new(FilesProxyModel::default())),
        }
    }
}
