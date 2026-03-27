use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::{cell::RefCell, process, rc::Rc};

use crate::{
    model::{CommitProxyModels, RepositoriesProxyModels},
    worker::Worker,
};

mod controller;
mod model;
mod storage;

mod command_utils;
mod git_utils;
mod repositories;
mod worker;

pub mod ui;

struct AppProxyModels {
    commit_proxy_models: Rc<CommitProxyModels>,
    repositories_proxy_models: Rc<RefCell<RepositoriesProxyModels>>,
}

impl AppProxyModels {
    fn new(app_window: &ui::AppWindow) -> Self {
        let commit_model: ModelRc<ui::SlintCommit> = Rc::new(VecModel::default()).into();
        app_window
            .global::<ui::SlintCommitPickerAdapter>()
            .set_commit_source_model(commit_model.clone());

        let author_model: ModelRc<SharedString> = Rc::new(VecModel::default()).into();
        app_window.global::<ui::SlintCommitPickerAdapter>().set_author_model(author_model);

        let commit_proxy_models = Rc::new(CommitProxyModels::new(commit_model));
        let repositories_proxy_models = Rc::new(RefCell::new(RepositoriesProxyModels::new()));

        Self {
            commit_proxy_models,
            repositories_proxy_models,
        }
    }
}

pub fn main() {
    let app_window = ui::AppWindow::new().expect("Error while creating app window!");

    let app_proxy_models = AppProxyModels::new(&app_window);

    let worker = Worker::new(&app_window);

    app_window.on_close(move || process::exit(0));

    controller::setup_review_helper_settings(&app_window, worker.channel.clone());

    controller::setup_review_helper(&app_window, worker.channel.clone());

    controller::setup_repository_callbacks(&app_window, worker.channel.clone());

    controller::setup_review_callbacks(&app_window, worker.channel.clone(), app_proxy_models.repositories_proxy_models.clone());

    controller::setup_utils(&app_window);

    controller::setup_commit_picker(&app_window, app_proxy_models.commit_proxy_models.clone(), worker.channel.clone());

    controller::setup_file_picker(&app_window, app_proxy_models.repositories_proxy_models.clone());

    controller::setup_file_diffs(&app_window);

    app_window.run().unwrap();
    worker.join().unwrap();
}
