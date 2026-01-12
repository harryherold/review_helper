use std::{path::PathBuf, rc::Rc};

use crate::{
    git_utils,
    model::{CommitProxyModel, IdModel},
    repositories::{self, RepositoryId},
    ui::{self, SlintCommit},
    worker::{WorkerChannel, WorkerMessage},
};
use slint::{ComponentHandle, Model, ModelRc};

pub fn setup_commit_picker(app_window: &ui::AppWindow, commit_proxy_model: Rc<CommitProxyModel>, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintCommitPickerAdapter>().on_ui_commit_model({
        let commit_proxy_model = commit_proxy_model.clone();
        move || -> ModelRc<SlintCommit> { commit_proxy_model.ui_model() }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_filter_commits({
        let commit_proxy_model = commit_proxy_model.clone();
        move |pattern| {
            commit_proxy_model.set_filter_text(pattern);
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_refresh({
        let channel = worker_channel.clone();
        move |repository_id| {
            let message = WorkerMessage::QueryCommits(RepositoryId::from(repository_id));
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_index_of_merge_base({
        let commit_proxy_model = commit_proxy_model.clone();
        let ui_weak = app_window.as_weak();
        move |repository_id| -> i32 {
            let ui = ui_weak.unwrap();
            let repositories = ui.global::<ui::SlintReviewHelper>().get_repositories();
            let repositories = repositories.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
            let Some(repository) = repositories.get(repository_id as usize) else {
                return -1;
            };
            let path = PathBuf::from(repository.path.as_str());
            let base_branch = repository.base_branch.as_str();
            let Ok(feature_branch) = git_utils::current_branch(&path) else {
                return -1;
            };
            let Ok(commit_id) = git_utils::branch_merge_base(&path, base_branch, feature_branch.as_str()) else {
                return -1;
            };
            match commit_proxy_model.ui_model().iter().position(|c| commit_id.contains(c.commit_id.as_str())) {
                Some(index) => index as i32,
                None => -1,
            }
        }
    });
}
