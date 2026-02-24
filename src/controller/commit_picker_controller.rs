use std::{path::PathBuf, rc::Rc};

use crate::{
    git_utils,
    model::{CommitProxyModels, IdModel, model_utils},
    repositories::RepositoryId,
    ui::{self, SlintCommit, SlintResult},
    worker::{WorkerChannel, WorkerMessage},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString};

fn query_merge_base(app_window: &ui::AppWindow, repository_id: usize) -> Option<SharedString> {
    let repositories = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let repositories = repositories.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
    let Some(repository) = repositories.get(repository_id) else {
        return None;
    };
    let path = PathBuf::from(repository.path.as_str());
    let base_branch = repository.base_branch.as_str();
    let Ok(feature_branch) = git_utils::current_branch(&path) else {
        return None;
    };

    match git_utils::branch_merge_base(&path, base_branch, feature_branch.as_str()) {
        Ok(commit_hash) => Some(SharedString::from(&commit_hash)),
        Err(_) => {
            model_utils::report_error(
                app_window,
                SlintResult::QueryingMergeBaseFailed,
                SharedString::from(format!("base branch {}", base_branch)),
            );
            None
        }
    }
}

pub fn setup_commit_picker(app_window: &ui::AppWindow, commit_proxy_model: Rc<CommitProxyModels>, worker_channel: WorkerChannel) {
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
    app_window.global::<ui::SlintCommitPickerAdapter>().on_sort_commits({
        let commit_proxy_model = commit_proxy_model.clone();
        move |criterion, is_sort_ascending| {
            commit_proxy_model.set_sort_criteria(criterion, is_sort_ascending);
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_refresh({
        let channel = worker_channel.clone();
        move |repository_id| {
            let message = WorkerMessage::QueryCommits(RepositoryId::from(repository_id));
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_merge_base({
        let ui_weak = app_window.as_weak();
        move |repository_id| -> SharedString {
            let ui = ui_weak.unwrap();
            match query_merge_base(&ui, repository_id as usize) {
                Some(merge_base) => merge_base,
                None => SharedString::new(),
            }
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_index_of_merge_base({
        let commit_proxy_model = commit_proxy_model.clone();
        let ui_weak = app_window.as_weak();
        move |repository_id| -> i32 {
            let ui = ui_weak.unwrap();

            let Some(commit_hash) = query_merge_base(&ui, repository_id as usize) else {
                return -1;
            };

            match commit_proxy_model.ui_model().iter().position(|c| commit_hash.contains(c.commit_id.as_str())) {
                Some(index) => index as i32,
                None => -1,
            }
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_commit_message_of({
        let commit_proxy_model = commit_proxy_model.clone();
        move |commit_hash| -> SharedString {
            match commit_proxy_model.ui_model().iter().find(|c| commit_hash.contains(c.commit_id.as_str())) {
                Some(commit) => commit.message,
                None => SharedString::new(),
            }
        }
    });
}
