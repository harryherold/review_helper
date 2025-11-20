use std::{cell::RefCell, rc::Rc};

use crate::{controller::review_helper_controller::report_error, model::AppState, storage::RepositoryName, ui};

use slint::ComponentHandle;

pub fn setup_repository_callbacks(app_state: Rc<RefCell<AppState>>) {
    app_state.borrow().app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
        let state = app_state.clone();
        let ui_weak = app_state.borrow().app_window.as_weak();
        move |id, name| {
            let ui = ui_weak.unwrap();
            let name = RepositoryName::from(name.as_str());
            let review_helper = &state.borrow().review_helper;
            // TODO Should be async!
            match review_helper.storage.clone().load_review_names(&name) {
                Ok(names) => {
                    if let Some(repository) = review_helper.repositories.get(&name) {
                        repository.update(names);
                    } else {
                        report_error(review_helper, &ui, crate::model::ReviewHelperError::ModelItemNotExists);
                    }
                }
                Err(e) => {
                    report_error(review_helper, &ui, crate::model::ReviewHelperError::LoadReviewNamesFailed(e.to_string()));
                }
            }
        }
    });
}
