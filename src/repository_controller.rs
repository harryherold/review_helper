use crate::app_state::AppState;
use crate::command_utils::run_command;
use crate::ui;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Model, SharedString};
use std::path::PathBuf;

pub fn setup_repository(app_state: &AppState) {
    app_state.app_window.global::<ui::Repository>().on_open({
        let ui_weak = app_state.app_window.as_weak();
        let project_ref = app_state.project.clone();
        move || {
            let ui = ui_weak.unwrap();
            let mut project_ref = project_ref.borrow_mut();
            match FileDialog::new().set_location("~").show_open_single_dir().unwrap() {
                Some(repo_path) => {
                    if let Some(old_path) = project_ref.repository.repository_path() {
                        if old_path == repo_path.to_str().expect("Could not convert path to string!") {
                            return;
                        }
                    }
                    ui.global::<ui::Project>().set_has_modifications(true);
                    if let Some(path) = repo_path.to_str() {
                        ui.global::<ui::Repository>().set_path(SharedString::from(path));
                    }
                    project_ref.repository.set_path(repo_path);
                }
                None => {}
            }
        }
    });
    app_state.app_window.global::<ui::Diff>().on_filter_file_diff({
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        move |pattern| {
            let mut m = file_diff_model_ctx.borrow_mut();
            m.set_filter_text(pattern);
        }
    });
    app_state.app_window.global::<ui::Diff>().on_set_filter_review_state({
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        let ui_weak = app_state.app_window.as_weak();
        move |filter_review_state| {
            let mut m = file_diff_model_ctx.borrow_mut();
            m.set_filter_review_state(filter_review_state);
            
            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_current_filter_review_state(filter_review_state);
        }
    });
    app_state.app_window.global::<ui::Diff>().on_diff_start_end({
        let ui_weak = app_state.app_window.as_weak();
        let project_ref = app_state.project.clone();
        move |start_commit, end_commit| {
            let (old_start_commit, old_end_commit) = {
                let project = project_ref.borrow();
                let (old_start, old_end) = project.repository.diff_range();
                (SharedString::from(old_start), SharedString::from(old_end))
            };
            let result = project_ref.borrow_mut().repository.diff_repository(&start_commit, &end_commit);
            if let Err(error) = result {
                eprintln!("Error on diffing repo: {}", error.to_string());
                return;
            }

            let ui = ui_weak.unwrap();
            if old_start_commit != start_commit || old_end_commit != end_commit {
                ui.global::<ui::Project>().set_has_modifications(true);
            }
            ui.global::<ui::Diff>().set_start_commit(start_commit);
            ui.global::<ui::Diff>().set_end_commit(end_commit);
            let project = project_ref.borrow();
            let statistics = project.repository.statistics();

            ui.global::<ui::OverallDiffStats>().set_added_lines(statistics.added_lines as i32);
            ui.global::<ui::OverallDiffStats>().set_removed_lines(statistics.removed_lines as i32);
        }
    });
    app_state.app_window.global::<ui::Diff>().on_open_file_diff({
        let project_ref = app_state.project.clone();
        let app_config = app_state.app_config.clone();
        move |id| {
            if let Err(error) = project_ref.borrow().repository.diff_file(id, &app_config.borrow().config.diff_tool) {
                eprintln!("Error occurred while file diff: {}", error.to_string())
            }
        }
    });
    app_state.app_window.global::<ui::Diff>().on_open_file({
        let project_ref = app_state.project.clone();
        let app_config = app_state.app_config.clone();
        move |file_path| {
            let project = project_ref.borrow();
            let repo_path = project.repository.repository_path().expect("Repository path is not set!");
            let app_config = app_config.borrow();
            let args = app_config
                .config
                .editor_args
                .iter()
                .map(|arg| {
                    if arg.contains("{file}") {
                        arg.replace("{file}", file_path.as_str())
                    } else {
                        arg.to_string()
                    }
                })
                .collect::<Vec<String>>();
            if let Err(error) = run_command(&app_config.config.editor, &args, &PathBuf::from(repo_path)) {
                eprintln!("Error occurred while opening file: {}", error.to_string())
            }
        }
    });
    app_state.app_window.global::<ui::Diff>().on_toggle_is_reviewed({
        let project_ref = app_state.project.clone();
        move |id| project_ref.borrow_mut().repository.toggle_file_is_reviewed(id as usize)
    });
    app_state.app_window.global::<ui::Diff>().on_set_sort_criteria({
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        let ui_weak = app_state.app_window.as_weak();
        move |sort_criteria| {
            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_current_sort_criteria(sort_criteria);
            file_diff_model_ctx.borrow_mut().sort_by(sort_criteria);
            let m = file_diff_model_ctx.borrow();
            ui.global::<ui::Diff>().set_diff_model(m.sort_model());
        }
    });
    app_state.app_window.global::<ui::Diff>().on_diff_model_contains_id({
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        move |id| -> bool {
            file_diff_model_ctx.borrow().sort_model().iter().any(|item| item.id == id)
        }
    })
}
