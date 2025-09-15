use slint::ComponentHandle;

use crate::app_state::AppState;
use crate::git_command_spawner::*;
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
}
