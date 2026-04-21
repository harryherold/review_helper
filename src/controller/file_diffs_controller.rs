use std::path::PathBuf;

use crate::command_utils;
use crate::model::{IdModel, model_utils};
use crate::ui;

use slint::{ComponentHandle, Model, SharedString};

const FILE_PLACEHOLER_STRING: &str = "{file}";
const EDITOR_ARGUMENT_SEPARATOR: &str = ",";

pub fn setup_file_diffs(app_window: &ui::AppWindow) {
    app_window.global::<ui::SlintFileDiffs>().on_open_file_by_editor({
        let app_window_weak = app_window.as_weak();
        move |repository_id, file_path| {
            let app_window = app_window_weak.unwrap();
            let settings = app_window.global::<ui::SlintReviewHelperSettings>();
            let editor = settings.get_editor().to_string();
            let editor_args = {
                let args = settings.get_editor_args();
                args.split(EDITOR_ARGUMENT_SEPARATOR)
                    .map(|arg| {
                        if arg.contains(FILE_PLACEHOLER_STRING) {
                            file_path.to_string()
                        } else {
                            arg.to_string()
                        }
                    })
                    .collect::<Vec<String>>()
            };
            let repository_path = {
                let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                let repository = repository_model.get(repository_id as usize).unwrap();
                PathBuf::from(repository.path.as_str())
            };
            if let Err(e) = command_utils::run_command(&editor, &editor_args, &repository_path) {
                model_utils::report_error(&app_window, ui::SlintResult::OpenEditorFailed, SharedString::from(e.to_string()));
            }
        }
    });
}
