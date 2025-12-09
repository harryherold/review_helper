use crate::{
    model::IdModel,
    review_helper_cache::ReviewId,
    storage::{RepositoryName, repository_storage::ReviewName},
    ui,
    worker::WorkerChannel,
};

use slint::{ComponentHandle, Model};

pub fn setup_review_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintReviewCallbacks>().on_load_review({
        let channel = worker_channel.clone();
        move |repository_id, repository_name, review_id| {
            channel
                .send(crate::worker::WorkerMessage::LoadReview {
                    repository_id: repository_id as usize,
                    repository_name: RepositoryName::from(repository_name.as_str()),
                    review_id: ReviewId::from(review_id),
                })
                .unwrap();
        }
    });
    app_window
        .global::<ui::SlintReviewCallbacks>()
        .on_review_id_to_index(move |review_id, review_model| -> i32 {
            if review_id <= 0 {
                return -1;
            }

            let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
            review_model.id_to_index(review_id as usize)
        });
}
