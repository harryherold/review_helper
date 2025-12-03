use std::fs;
use std::rc::Rc;

use slint::{ComponentHandle, ModelExt};

use crate::model::ReviewHelperCache;
use crate::storage::ReviewHelperFileStorage;
use crate::ui;

pub struct AppState {
    pub app_window: ui::AppWindow,
    // pub review_helper: ReviewHelper,
}

impl AppState {
    pub fn new() -> Self {
        // let mut app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
        // app_data_path.push(std::env!("CARGO_CRATE_NAME"));
        // if !app_data_path.exists() {
        //     let result = fs::create_dir(&app_data_path);
        //     assert!(result.is_ok());
        // }

        // let storage = ReviewHelperFileStorage::new(app_data_path);

        // let review_helper = ReviewHelper::new(Rc::new(storage));

        let app_window = ui::AppWindow::new().expect("Error while creating app window!");

        AppState { app_window }
    }
}
