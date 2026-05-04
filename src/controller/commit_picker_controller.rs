use std::{path::PathBuf, rc::Rc};

use crate::{
    cast_model, git_utils,
    model::{CommitProxyModels, IdModel, model_utils},
    repositories::RepositoryId,
    ui::{self, SlintCommit, SlintResult},
    unwrap_or_return,
    worker::{WorkerChannel, WorkerMessage},
};
use slint::{ComponentHandle, Model, ModelRc, SharedString};

fn query_merge_base(app_window: &ui::AppWindow, repository_id: usize) -> Option<SharedString> {
    let repositories = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let repositories = cast_model!(repositories, IdModel<ui::SlintRepository>);
    let repository = repositories.get(repository_id)?;
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
        move |pattern, filter_type| {
            commit_proxy_model.set_filter_text(pattern, filter_type);
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
            channel.send(message).expect("Worker channel broken!");
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_merge_base({
        let ui_weak = app_window.as_weak();
        move |repository_id| -> SharedString {
            let ui = unwrap_or_return!(ui_weak.upgrade(), "Upgrade to AppWindow failed!", SharedString::new());
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
            let ui = unwrap_or_return!(ui_weak.upgrade(), "Upgrade to AppWindow failed!", -1);

            let Some(commit_hash) = query_merge_base(&ui, repository_id as usize) else {
                return -1;
            };

            match commit_proxy_model.ui_model().iter().position(|c| commit_hash.contains(c.commit_id.as_str())) {
                Some(index) => index as i32,
                None => -1,
            }
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_index_of_commit({
        let commit_proxy_model = commit_proxy_model.clone();
        move |commit_hash| -> i32 {
            match commit_proxy_model.ui_model().iter().position(|c| commit_hash.contains(c.commit_id.as_str())) {
                Some(index) => index as i32,
                None => -1,
            }
        }
    });
    app_window.global::<ui::SlintCommitPickerAdapter>().on_commit_message_of({
        let ui_weak = app_window.as_weak();
        move |commit_hash| -> SharedString {
            let ui = unwrap_or_return!(ui_weak.upgrade(), "Upgrade to AppWindow failed!", SharedString::new());
            let commit_model = ui.global::<ui::SlintCommitPickerAdapter>().get_commit_source_model();
            match commit_model.iter().find(|c| commit_hash.contains(c.commit_id.as_str())) {
                Some(commit) => commit.message,
                None => SharedString::new(),
            }
        }
    });
}
