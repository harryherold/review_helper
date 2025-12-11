use crate::{review_helper_cache::RepositoryId, ui, worker::WorkerChannel};

use slint::ComponentHandle;

pub fn setup_repository_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintRepositoryCallbacks>().on_repository_changed({
        let channel = worker_channel.clone();
        move |id, base_branch| {
            let base_branch = String::from(base_branch);
            let id = RepositoryId::from(id);
            channel.send(crate::worker::WorkerMessage::ChangeRepository { id, base_branch }).unwrap();
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
        let channel = worker_channel.clone();
        move |id| {
            channel
                .send(crate::worker::WorkerMessage::LoadReviewNames { id: RepositoryId::from(id) })
                .unwrap();
        }
    });
}
