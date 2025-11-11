use std::fs;

use slint::ComponentHandle;

use crate::model::{ReviewHelperModel, ReviewHelperSettings};
use crate::storage::ReviewHelperFileStorage;
use crate::ui;

pub struct AppState {
    pub app_window: ui::AppWindow,
    pub review_helper_settings: ReviewHelperSettings,
    pub model: ReviewHelperModel,
}

impl AppState {
    pub fn new() -> Self {
        let mut app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
        app_data_path.push(std::env!("CARGO_CRATE_NAME"));
        if !app_data_path.exists() {
            let result = fs::create_dir(&app_data_path);
            assert!(result.is_ok());
        }

        let review_helper_settings = match ReviewHelperSettings::new(app_data_path.clone()) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e.to_string());
                ReviewHelperSettings::default()
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
        }
    }
}
