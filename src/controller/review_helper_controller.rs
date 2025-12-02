use native_dialog::FileDialog;

use slint::ComponentHandle;

use crate::ui;
use crate::worker::WorkerChannel;

// TODO initialize commit model
pub fn setup_review_helper(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
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

    app_window.global::<ui::SlintReviewHelper>().on_new_repository({
        let channel = worker_channel.clone();
        move || {
            if let Some(repository_path) = FileDialog::new()
                .set_location("~")
                .show_open_single_dir()
                .expect("Could not create FileDialog! Check your dependencies!")
            {
                channel.send(crate::worker::WorkerMessage::NewRepository(repository_path)).unwrap();
            }
            ui::SlintResult::Ok
        }
    });
}
