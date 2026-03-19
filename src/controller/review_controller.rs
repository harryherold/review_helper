use std::{cell::RefCell, rc::Rc};

use crate::{
    model::{FileDiffProxyModels, IdModel, NotesProxyModels, ProxyModels, ReviewProxyModels, model_utils},
    repositories::{FileDiffId, NoteId, RepositoryId, ReviewId},
    storage::repository_storage::{DiffRangeStore, ReviewName},
    ui,
    worker::{NoteChangeType, ReviewContentChange, WorkerChannel, WorkerMessage},
};

use regex::Regex;
use slint::{ComponentHandle, Model, ModelRc, SharedString};

fn is_vaild_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let re = Regex::new(r"^[A-Za-z][A-Za-z0-9]*$").unwrap();
    re.is_match(name)
}

pub fn setup_review_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel, proxy_models: Rc<RefCell<ProxyModels>>) {
    fn get_file_diff_proxy_model(ids: ui::SlintReviewIdParameters, proxy_models: &Rc<RefCell<ProxyModels>>) -> Rc<FileDiffProxyModels> {
        let repository_id = RepositoryId::from(ids.repository_id);
        let review_id = ReviewId::from(ids.review_id);

        let proxy_models = proxy_models.borrow();

        let repository_proxy_models = proxy_models.repository_proxy_models(&repository_id).expect("Could not find repository!");

        repository_proxy_models
            .review_proxy_models(&review_id)
            .expect("Could not find review!")
            .file_diff_proxy_model()
    }
    fn get_notes_proxy_models(ids: ui::SlintReviewIdParameters, proxy_models: &Rc<RefCell<ProxyModels>>) -> Rc<NotesProxyModels> {
        let repository_id = RepositoryId::from(ids.repository_id);
        let review_id = ReviewId::from(ids.review_id);

        let proxy_models = proxy_models.borrow();

        let repository_proxy_models = proxy_models.repository_proxy_models(&repository_id).expect("Could not find repository!");

        repository_proxy_models
            .review_proxy_models(&review_id)
            .expect("Could not find review!")
            .notes_proxy_model()
    }

    app_window.global::<ui::SlintReviewCallbacks>().on_file_diff_ui_model({
        let proxy_models = proxy_models.clone();
        move |ids| -> ModelRc<ui::SlintFileDiff> {
            let file_diff_proxy_model = get_file_diff_proxy_model(ids, &proxy_models);
            file_diff_proxy_model.ui_model()
        }
    });

    app_window.global::<ui::SlintReviewCallbacks>().on_set_file_diff_file_pattern({
        let proxy_models = proxy_models.clone();
        move |ids, file_pattern| {
            let file_diff_proxy_model = get_file_diff_proxy_model(ids, &proxy_models);
            file_diff_proxy_model.set_filter_pattern(file_pattern);
        }
    });

    app_window.global::<ui::SlintReviewCallbacks>().on_set_file_diff_review_state({
        let proxy_models = proxy_models.clone();
        move |ids, filter_review_state| {
            let file_diff_proxy_model = get_file_diff_proxy_model(ids, &proxy_models);
            file_diff_proxy_model.set_filter_review_state(filter_review_state);
        }
    });

    app_window.global::<ui::SlintReviewCallbacks>().on_set_file_diff_sort_criteria({
        let proxy_models = proxy_models.clone();
        move |ids, sort_criteria| {
            let file_diff_proxy_model = get_file_diff_proxy_model(ids, &proxy_models);
            file_diff_proxy_model.set_sort_by(sort_criteria);
        }
    });

    app_window.global::<ui::SlintReviewCallbacks>().on_initialize_ui_models({
        let ui_weak = app_window.as_weak();
        let proxy_models = proxy_models.clone();
        move |ids| {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);

            let mut proxy_models = proxy_models.borrow_mut();

            if !proxy_models.has_repository_proxy_models(&repository_id) {
                proxy_models.add_repository_proxy_models(repository_id.clone());
            }

            let repository_proxy_models = proxy_models.mut_repository_proxy_models(&repository_id).expect("Add repository failed!");

            if !repository_proxy_models.has_review_proxy_models(&review_id) {
                let ui = ui_weak.unwrap();

                if let Some(review) = model_utils::get_slint_review(&ui, repository_id.as_usize(), review_id.as_usize()) {
                    let review_proxy_models = ReviewProxyModels::new(review.file_diff_model.clone(), review.note_model.clone());
                    repository_proxy_models.add_review_proxy_models(review_id.clone(), review_proxy_models);
                } else {
                    model_utils::report_error(
                        &ui,
                        ui::SlintResult::ModelItemNotExists,
                        SharedString::from(format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize())),
                    );
                }
            }
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_load_review({
        let channel = worker_channel.clone();
        move |ids| {
            channel
                .send(crate::worker::WorkerMessage::LoadReview {
                    repository_id: RepositoryId::from(ids.repository_id),
                    review_id: ReviewId::from(ids.review_id),
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
    app_window.global::<ui::SlintReviewCallbacks>().on_change_file_diff_is_reviewed({
        let channel = worker_channel.clone();
        move |ids, new_is_reviewed| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let content_change = ReviewContentChange::FileDiffChange {
                file_diff_id: FileDiffId::from(ids.file_diff_id),
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
                note_id,
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
                note_id,
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
                note_id,
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
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_delete_note({
        let channel = worker_channel.clone();
        move |ids| {
            let message = WorkerMessage::DeleteNote {
                repository_id: RepositoryId::from(ids.review_id_parameters.repository_id),
                review_id: ReviewId::from(ids.review_id_parameters.review_id),
                note_id: NoteId::from(ids.note_id),
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_add_note({
        let channel = worker_channel.clone();
        move |ids, note_text, note_context| {
            let message = WorkerMessage::AddNote {
                repository_id: RepositoryId::from(ids.repository_id),
                review_id: ReviewId::from(ids.review_id),
                text: String::from(note_text.as_str()),
                context: String::from(note_context.as_str()),
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_note_ui_model({
        let proxy_models = proxy_models.clone();
        move |ids| -> ModelRc<ui::SlintNote> {
            let notes_proxy_model = get_notes_proxy_models(ids, &proxy_models);
            notes_proxy_model.ui_model()
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_set_notes_text_filter({
        let proxy_models = proxy_models.clone();
        move |ids, text_pattern| {
            let notes_proxy_model = get_notes_proxy_models(ids, &proxy_models);
            notes_proxy_model.set_text_filter(text_pattern);
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_set_notes_context_filter({
        let proxy_models = proxy_models.clone();
        move |ids, context_pattern| {
            let notes_proxy_model = get_notes_proxy_models(ids, &proxy_models);
            notes_proxy_model.set_context_filter(context_pattern);
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_set_notes_sort_parameter({
        let proxy_models = proxy_models.clone();
        move |ids, criteria, order| {
            let notes_proxy_model = get_notes_proxy_models(ids, &proxy_models);
            notes_proxy_model.set_sort_parameter(criteria, order);
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_exists_file_diff({
        let app_window_weak = app_window.as_weak();
        move |file_diff_id_parameter| -> bool {
            let app_window = app_window_weak.unwrap();
            let model = model_utils::get_file_diff_model(
                &app_window,
                file_diff_id_parameter.review_id_parameters.repository_id as usize,
                file_diff_id_parameter.review_id_parameters.review_id as usize,
            );
            let model = model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
            model.has(file_diff_id_parameter.file_diff_id as usize)
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_change_review_name({
        let channel = worker_channel.clone();
        move |ids, new_review_name| {
            let message = WorkerMessage::ChangeReview {
                repository_id: RepositoryId::from(ids.repository_id),
                review_id: ReviewId::from(ids.review_id),
                content_change: ReviewContentChange::NameChange(ReviewName::from(new_review_name.as_str())),
            };
            channel.send(message).unwrap();
        }
    });
    app_window.global::<ui::SlintReviewCallbacks>().on_is_valid_review_name({
        let app_window_weak = app_window.as_weak();
        move |ids, name| -> bool {
            if !is_vaild_name(name.as_str()) {
                return false;
            }
            let app_window = app_window_weak.unwrap();
            let review_model = model_utils::get_review_model(&app_window, ids.repository_id as usize);
            let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
            !review_model.iter().any(|review| {
                if ids.review_id == 0 {
                    review.name == name
                } else {
                    review.name == name && review.id != ids.review_id
                }
            })
        }
    });
}
