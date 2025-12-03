use native_dialog::FileDialog;

use slint::ComponentHandle;

use crate::ui;
use crate::worker::WorkerChannel;

pub fn setup_review_helper(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
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
