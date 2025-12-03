use crate::{storage::RepositoryName, ui, worker::WorkerChannel};

use slint::ComponentHandle;

pub fn setup_repository_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintRepositoryCallbacks>().on_repository_changed({
        let channel = worker_channel.clone();
        move |repository_name, base_branch| {
            let name = RepositoryName::from(repository_name.as_str());
            let base_branch = String::from(base_branch);
            channel.send(crate::worker::WorkerMessage::ChangeRepository { name, base_branch }).unwrap();
        }
    });
    // app_state.borrow().app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
    //     let state = app_state.clone();
    //     let ui_weak = app_state.borrow().app_window.as_weak();
    //     move |id, name| {
    //         let ui = ui_weak.unwrap();
    //         let name = RepositoryName::from(name.as_str());
    //         let review_helper = &state.borrow().review_helper;
    //         // TODO Should be async!
    //         match review_helper.storage.clone().load_review_names(&name) {
    //             Ok(names) => {
    //                 if let Some(repository) = review_helper.repositories.get(&name) {
    //                     repository.update(names);
    //                 } else {
    //                     report_error(review_helper, &ui, crate::model::ReviewHelperError::ModelItemNotExists);
    //                 }
    //             }
    //             Err(e) => {
    //                 report_error(review_helper, &ui, crate::model::ReviewHelperError::LoadReviewNamesFailed(e.to_string()));
    //             }
    //         }
    //     }
    // });
}
