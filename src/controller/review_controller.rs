use crate::{
    model::IdModel,
    review_helper_cache::{RepositoryId, ReviewId},
    ui,
    worker::WorkerChannel,
};

use slint::{ComponentHandle, Model};

pub fn setup_review_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintReviewCallbacks>().on_load_review({
        let channel = worker_channel.clone();
        move |repository_id, review_id| {
            channel
                .send(crate::worker::WorkerMessage::LoadReview {
                    repository_id: RepositoryId::from(repository_id),
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
    app_window.global::<ui::SlintReviewCallbacks>().on_new_review({
        let channel = worker_channel.clone();
        move |id, name| {
            channel.send(crate::worker::WorkerMessage::NewReview { repository_id: RepositoryId::from(id), name: String::from(&name) }).unwrap();
        }
    })
}
