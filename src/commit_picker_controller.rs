use std::cell::RefCell;
use std::future::Future;
use std::path::PathBuf;
use std::rc::Rc;

use crate::project::Project;

use slint::ComponentHandle;

use crate::app_state::AppState;
use crate::git_utils;
use crate::ui;

fn spawn_query_commits_task(project: Rc<RefCell<Project>>) {
    slint::spawn_local(async move {
        let path = {
            let p = project.borrow();
            let path_str = p.repository.repository_path().expect("No repository path available!");
            PathBuf::from(path_str)
        };
        let commits = tokio::spawn(async move { git_utils::query_commits(&path).expect("Could not query commits!") })
            .await
            .expect("tokio spawn query_commits failed!");
        let mut p = project.borrow_mut();
        p.repository.set_commit_history(commits);
    })
    .expect("spawn_local failed!");
}

pub fn setup_commit_picker(app_state: &AppState) {
    app_state.app_window.global::<ui::CommitPickerAdapter>().on_refresh({
        let project = app_state.project.clone();
        move || {
            let project = project.clone();
            spawn_query_commits_task(project);
        }
    });
    app_state.app_window.global::<ui::CommitPickerAdapter>().on_filter_commits({
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        move |pattern| {
            let mut m = commit_proxy_model.borrow_mut();
            m.set_filter_text(pattern);
        }
    });
    app_state.app_window.global::<ui::CommitPickerAdapter>().on_sort_commits({
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        let ui_weak = app_state.app_window.as_weak();
        move |sort_index, is_sort_ascending| {
            let ui = ui_weak.unwrap();
            let mut m = commit_proxy_model.borrow_mut();
            m.sort_by(sort_index as usize, is_sort_ascending);
            ui.global::<ui::CommitPickerAdapter>().set_commit_model(m.sort_model());
        }
    });
}
