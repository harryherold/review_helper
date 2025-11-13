use slint::ComponentHandle;
use std::{cell::RefCell, process, rc::Rc};

use tokio::runtime::Runtime;

use crate::model::AppState;

mod controller;
mod model;
mod storage;

mod command_utils;
mod git_command_spawner;
mod git_utils;

pub mod ui;

pub fn main() -> Result<(), slint::PlatformError> {
    let rt = Runtime::new().unwrap();

    let _guard = rt.enter();

    let app_state = Rc::new(RefCell::new(AppState::new()));

    app_state.borrow().app_window.on_close(move || process::exit(0));

    controller::setup_review_helper_settings(app_state.clone());
    controller::setup_review_helper(app_state.clone());
    controller::setup_utils(app_state.clone());

    let ui = app_state.borrow().app_window.as_weak();
    ui.unwrap().run()
}
