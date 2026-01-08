use crate::{
    repositories::{RepositoryId, ReviewId},
    ui,
    worker::{WorkerChannel, WorkerMessage},
};

use slint::ComponentHandle;

pub fn setup_repository_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintRepositoryCallbacks>().on_repository_changed({
        let channel = worker_channel.clone();
        move |id, base_branch| {
            let base_branch = String::from(base_branch);
            let id = RepositoryId::from(id);
            channel.send(WorkerMessage::ChangeRepository { id, base_branch }).unwrap();
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
        let channel = worker_channel.clone();
        move |id| {
            channel.send(WorkerMessage::LoadReviewNames { id: RepositoryId::from(id) }).unwrap();
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_new_review({
        let channel = worker_channel.clone();
        move |id, name| {
            channel
                .send(crate::worker::WorkerMessage::NewReview {
                    repository_id: RepositoryId::from(id),
                    name: String::from(&name),
                })
                .unwrap();
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_delete_review({
        let channel = worker_channel.clone();
        move |ids| {
            let message = crate::worker::WorkerMessage::DeleteReview {
                repository_id: RepositoryId::from(ids.repository_id),
                review_id: ReviewId::from(ids.review_id),
            };

            channel.send(message).unwrap();
        }
    });
}
