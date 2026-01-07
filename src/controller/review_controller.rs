use crate::{
    model::IdModel,
    repositories::{FileDiffId, NoteId, RepositoryId, ReviewId},
    storage::repository_storage::DiffRangeStore,
    ui,
    worker::{NoteChangeType, ReviewContentChange, WorkerChannel, WorkerMessage},
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
            channel
                .send(crate::worker::WorkerMessage::NewReview {
                    repository_id: RepositoryId::from(id),
                    name: String::from(&name),
                })
                .unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_change_file_diff_is_reviewed({
        let channel = worker_channel.clone();
        move |ids, new_is_reviewed| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let content_change = ReviewContentChange::FileDiffChange {
                id: FileDiffId::from(ids.file_diff_id),
                is_reviewed: new_is_reviewed,
            };
            let message = WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_change_note_text({
        let channel = worker_channel.clone();
        move |ids, new_text| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let note_id = NoteId::from(ids.note_id);
            let content_change = ReviewContentChange::NoteChange {
                id: note_id,
                change_type: NoteChangeType::TextChanged(String::from(&new_text)),
            };
            let message = WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_change_note_context({
        let channel = worker_channel.clone();
        move |ids, new_context| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let note_id = NoteId::from(ids.note_id);
            let content_change = ReviewContentChange::NoteChange {
                id: note_id,
                change_type: NoteChangeType::ContextChanged(String::from(&new_context)),
            };
            let message = WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_change_note_is_done({
        let channel = worker_channel.clone();
        move |ids, new_is_done| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let note_id = NoteId::from(ids.note_id);
            let content_change = ReviewContentChange::NoteChange {
                id: note_id,
                change_type: NoteChangeType::IsDoneChanged(new_is_done),
            };
            let message = WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_find_file_changes({
        let channel = worker_channel.clone();
        move |ids, diff_range| {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);
            let message = WorkerMessage::FindFileDifferences {
                repository_id,
                review_id,
                diff_range: DiffRangeStore {
                    start: String::from(diff_range.start.as_str()),
                    end: String::from(diff_range.end.as_str()),
                },
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_show_file_differences({
        let channel = worker_channel.clone();
        move |ids| {
            let message = WorkerMessage::ShowFileDifferences {
                repository_id: RepositoryId::from(ids.review_id_parameters.repository_id),
                review_id: ReviewId::from(ids.review_id_parameters.review_id),
                file_diff_id: FileDiffId::from(ids.file_diff_id),
            };
            channel.send(message).unwrap();
        }
    })
}
