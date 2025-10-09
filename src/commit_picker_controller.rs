use slint::{ComponentHandle, Model, SharedString};

use crate::app_state::AppState;
use crate::git_command_spawner::*;
use crate::git_utils::{branch_merge_base, current_branch};
use crate::ui;

pub fn setup_commit_picker(app_state: &AppState) {
    app_state.app_window.global::<ui::CommitPickerAdapter>().on_refresh({
        let project = app_state.project.clone();
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        move || {
            let project = project.borrow();
            if project.repository.path.is_none() {
                return;
            }
            async_query_commits(project.repository.path.as_ref().unwrap(), commit_proxy_model.clone());
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
    app_state.app_window.global::<ui::CommitPickerAdapter>().on_index_of_merge_base({
        let project = app_state.project.clone();
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        move |base_branch| -> i32 {
            let project = project.borrow();
            if project.repository.path.is_none() {
                return -1;
            }
            if commit_proxy_model.borrow().sort_model().row_count() == 0 {
                return -1;
            }
            let path = project.repository.path.as_ref().unwrap();
            let Ok(current_branch) = current_branch(path) else {
                eprintln!("Error occured while determining current branch!");
                return -1;
            };

            let commit_result = branch_merge_base(path, base_branch.as_str(), &current_branch);
            match commit_result {
                Err(e) => {
                    eprintln!("Error occured while determining merge base branch: {}", e.to_string());
                    -1
                }
                Ok(commit) => {
                    let c = SharedString::from(commit);
                    let model = commit_proxy_model.borrow().sort_model();

                    model
                        .iter()
                        .position(|item| c.contains(item.row_data(0).unwrap_or_default().text.as_str()))
                        .unwrap_or_default() as i32
                }
            }
        }
    });
}
