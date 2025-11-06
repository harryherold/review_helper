use slint::ComponentHandle;
use std::{cell::RefCell, process, rc::Rc};

use tokio::runtime::Runtime;

use crate::app_state::AppState;

mod storage;

mod app_config;
mod app_config_controller;
mod app_state;
mod command_utils;
mod commit_picker_controller;
mod commit_proxy_model;
mod file_diff_proxy_models;
mod files_proxy_model;
mod git_command_spawner;
mod git_utils;
mod id_model;
mod notes;
mod notes_controller;
mod notes_proxy_models;
mod project;
mod project_config;
mod project_controller;
mod repositories;
mod repositories_controller;
mod repository;
mod repository_controller;

mod utils_controller;

pub mod ui;

pub fn main() -> Result<(), slint::PlatformError> {
    let rt = Runtime::new().unwrap();

    let _guard = rt.enter();

    let app_state = Rc::new(RefCell::new(AppState::new()));

    app_state.borrow().app_window.on_close(move || process::exit(0));

    // project_controller::setup_project(&mut app_state);
    app_config_controller::setup_app_config(app_state.clone());
    repositories_controller::setup(app_state.clone());
    // repository_controller::setup_repository(&app_state);
    // commit_picker_controller::setup_commit_picker(&app_state);
    // notes_controller::setup_notes(&app_state);
    utils_controller::setup_utils(app_state.clone());

    let ui = app_state.borrow().app_window.as_weak();
    ui.unwrap().run()
}

fn setup_repositories(app_state: Rc<RefCell<AppState>>) {}
