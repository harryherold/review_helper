use slint::ComponentHandle;
use std::{cell::RefCell, process, rc::Rc};

use crate::{model::AppState, worker::Worker};

mod controller;
mod model;
mod storage;

mod command_utils;
mod git_command_spawner;
mod git_utils;
mod worker;

pub mod ui;

pub fn main() {
    let app_state = Rc::new(RefCell::new(AppState::new()));

    let app_window = &app_state.borrow().app_window;

    let worker = Worker::new(app_window);

    app_window.on_close(move || process::exit(0));

    controller::setup_review_helper_settings(app_window, worker.channel.clone());

    controller::setup_review_helper(app_state.clone());
    // controller::setup_repository_callbacks(app_state.clone());

    controller::setup_utils(app_state.clone());

    app_window.run().unwrap();
    worker.join().unwrap();
}
