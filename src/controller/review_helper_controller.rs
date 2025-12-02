use std::{cell::RefCell, rc::Rc};

use native_dialog::FileDialog;

use slint::{ComponentHandle, SharedString};

use crate::model::{AppState, IdModelChange, ReviewHelper, ReviewHelperError};
use crate::storage::RepositoryStore;
use crate::ui;

pub fn report_error(review_helper: &ReviewHelper, app_window: &ui::AppWindow, error: ReviewHelperError) {
    use crate::model::ReviewHelperError::*;

    let (ui_error, ui_error_text) = match error {
        RepositoryExists(t) => (ui::SlintResult::RepositoryExists, SharedString::from(t.as_str())),
        GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, SharedString::from(t.as_str())),
        NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, SharedString::from(t.as_str())),
        StoreFailed(t) => (ui::SlintResult::StoreFailed, SharedString::from(t.as_str())),
        ModelItemNotExists => (ui::SlintResult::ModelItemNotExists, SharedString::from("Model item does not exist!")),
        LoadReviewNamesFailed(t) => (ui::SlintResult::LoadReviewNamesFailed, SharedString::from(t.as_str())),
    };
    review_helper.add_error(ui_error, ui_error_text);
    app_window.invoke_request_show_error(ui_error);
}

// TODO initialize commit model
pub fn setup_review_helper(app_state: Rc<RefCell<AppState>>) {
    // app_state.borrow().review_helper.repositories_model.set_observer({
    //     let state = app_state.clone();
    //     let repositories_model = state.borrow().review_helper.repositories_model.clone();
    //     let storage = state.borrow().review_helper.storage.clone();
    //     let ui_weak = state.borrow().app_window.as_weak();

    //     move |change_type| {
    //         if let IdModelChange::EntityChanged(id) = change_type {
    //             let ui = ui_weak.unwrap();

    //             let repository_result = repositories_model.get(id);
    //             if repository_result.is_none() {
    //                 report_error(&state.borrow().review_helper, &ui, ReviewHelperError::ModelItemNotExists);
    //                 return;
    //             }
    //             let repository = repository_result.unwrap_or_default();
    //             let save_result = storage.save_repository(RepositoryStore::from(&repository));
    //             if let Err(e) = save_result {
    //                 report_error(&state.borrow().review_helper, &ui, ReviewHelperError::StoreFailed(e.to_string()));
    //             }
    //         }
    //     }
    // });

    // app_state.borrow().app_window.global::<ui::SlintReviewHelper>().on_new_repository({
    //     let state = app_state.clone();
    //     let ui_weak = state.borrow().app_window.as_weak();
    //     move || {
    //         if let Some(repository_path) = FileDialog::new()
    //             .set_location("~")
    //             .show_open_single_dir()
    //             .expect("Could not create FileDialog! Check your dependencies!")
    //         {
    //             let result = state.borrow_mut().review_helper.add_repository(repository_path);
    //             if let Err(e) = result {
    //                 let ui = ui_weak.unwrap();
    //                 report_error(&state.borrow().review_helper, &ui, e);
    //             }
    //         }
    //         ui::SlintResult::Ok
    //     }
    // });
}
