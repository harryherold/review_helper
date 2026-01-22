use slint::ComponentHandle;
use std::{cell::RefCell, process, rc::Rc};

use crate::{model::ProxyModels, worker::Worker};

mod controller;
mod model;
mod storage;

mod command_utils;
mod git_command_spawner;
mod git_utils;
mod repositories;
mod worker;

pub mod ui;

pub fn main() {
    let app_window = ui::AppWindow::new().expect("Error while creating app window!");

    let proxy_models = Rc::new(RefCell::new(ProxyModels::new(&app_window)));

    let worker = Worker::new(&app_window);

    app_window.on_close(move || process::exit(0));

    controller::setup_review_helper_settings(&app_window, worker.channel.clone());

    controller::setup_review_helper(&app_window, worker.channel.clone());

    controller::setup_repository_callbacks(&app_window, worker.channel.clone());

    controller::setup_review_callbacks(&app_window, worker.channel.clone(), proxy_models.clone());

    controller::setup_utils(&app_window);

    controller::setup_commit_picker(&app_window, proxy_models.borrow().commit_proxy_models.clone(), worker.channel.clone());

    controller::setup_file_picker(&app_window, proxy_models.clone());

    app_window.run().unwrap();
    worker.join().unwrap();
}
