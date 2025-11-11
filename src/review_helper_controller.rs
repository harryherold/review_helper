use std::{cell::RefCell, rc::Rc};

use native_dialog::FileDialog;

use slint::{ComponentHandle, SharedString};

use crate::app_state::AppState;
use crate::ui;

// TODO initialize commit model
// TODO track changes

pub fn setup(app_state: Rc<RefCell<AppState>>) {
    app_state.borrow().app_window.global::<ui::SlintReviewHelper>().on_new_repository({
        let state = app_state.clone();
        move || {
            if let Some(repository_path) = FileDialog::new()
                .set_location("~")
                .show_open_single_dir()
                .expect("Could not create FileDialog! Check your dependencies!")
            {
                let model = &mut state.borrow_mut().model;
                if let Err(e) = model.add_repository(repository_path) {
                    use crate::review_helper::ReviewHelperError::*;

                    let (ui_error, ui_error_text) = match e {
                        RepositoryExists(t) => (ui::SlintResult::RepositoryExists, SharedString::from(t.as_str())),
                        GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, SharedString::from(t.as_str())),
                        NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, SharedString::from(t.as_str())),
                        StoreFailed(t) => (ui::SlintResult::StoreFailed, SharedString::from(t.as_str())),
                    };
                    model.add_error(ui_error, ui_error_text);

                    return ui_error;
                }
            }
            ui::SlintResult::Ok
        }
    });
}
