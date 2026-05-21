use crate::{
    cast_model,
    model::IdModel,
    repositories::{RepositoryId, ReviewId},
    ui, unwrap_or_return,
    worker::{WorkerChannel, WorkerMessage},
};

use slint::{ComponentHandle, Model};

pub fn setup_repository_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintRepositoryCallbacks>().on_repository_changed({
        let channel = worker_channel.clone();
        move |id, base_branch| {
            let base_branch = String::from(base_branch);
            let id = RepositoryId::from(id);
            channel
                .send(WorkerMessage::ChangeRepository { id, base_branch })
                .expect("Worker channel broken!");
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
        let channel = worker_channel.clone();
        move |id| {
            channel
                .send(WorkerMessage::LoadRepository { id: RepositoryId::from(id) })
                .expect("Worker channel broken!");
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
                .expect("Worker channel broken!");
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_delete_review({
        let channel = worker_channel.clone();
        move |ids| {
            let message = crate::worker::WorkerMessage::DeleteReview {
                repository_id: RepositoryId::from(ids.repository_id),
                review_id: ReviewId::from(ids.review_id),
            };

            channel.send(message).expect("Worker channel broken!");
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_index_of_id({
        let app_window_weak = app_window.as_weak();
        move |id| -> i32 {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", -1);
            let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
            let repository_model = cast_model!(repository_model, IdModel<ui::SlintRepository>);
            repository_model.id_to_index(id as usize).map_or(-1, |i| i as i32)
        }
    });
}
