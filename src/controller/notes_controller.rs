use std::rc::Rc;

use slint::language::{DragAction, DropEvent};
use slint::{ComponentHandle, DataTransfer, Model, ModelRc};

use crate::model::{IdModel, model_utils};
use crate::repositories::{FileDiffId, NoteId, RepositoryId, ReviewId};
use crate::worker::WorkerChannel;
use crate::{cast_model, ui, unwrap_or_return};

#[derive(Clone)]
struct DragPayload {
    line_no: i32,
    note_id: i32,
}

pub fn setup_notes_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::DragFileLineNote>().on_make_data({
        move |line_no, note_id| {
            let mut transfer = DataTransfer::default();
            transfer.set_user_data(Rc::new(DragPayload { line_no, note_id }));
            transfer
        }
    });
    app_window.global::<ui::DragFileLineNote>().on_can_drop({
        move |event: DropEvent, target_line_no: i32| -> DragAction {
            if target_line_no < 0 {
                return DragAction::None;
            }

            if let Some(payload) = event.data.user_data().and_then(|rc| rc.downcast::<DragPayload>().ok())
                && payload.note_id != -1
                && payload.line_no != -1
            {
                return event.proposed_action;
            }
            DragAction::None
        }
    });
    app_window.global::<ui::DragFileLineNote>().on_dragged_note_id({
        move |event: DropEvent| -> i32 {
            if let Some(payload) = event.data.user_data().and_then(|rc| rc.downcast::<DragPayload>().ok()) {
                payload.note_id
            } else {
                -1
            }
        }
    });
    let note_callbacks = app_window.global::<ui::SlintNoteCallbacks>();
    note_callbacks.on_note_source_model({
        let app_window_weak = app_window.as_weak();
        move |ids| -> ModelRc<ui::SlintNote> {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", ModelRc::default());
            let review = model_utils::get_slint_review(&app_window, ids.repository_id as usize, ids.review_id as usize);
            let review = review.unwrap_or_else(|| panic!("[BUG] {}, {} not found", ids.repository_id, ids.review_id));
            review.note_model
        }
    });
    note_callbacks.on_index_of_id({
        move |id, note_model| -> i32 {
            let note_model = cast_model!(note_model, IdModel<ui::SlintNote>);
            note_model.id_to_index(id as usize).map(|i| i as i32).unwrap_or(-1)
        }
    });
    note_callbacks.on_add_code_line_note({
        let channel = worker_channel.clone();
        move |ids, note_text, line_content, line_number| {
            let message = crate::worker::WorkerMessage::AddNote {
                repository_id: RepositoryId::from(ids.review_id_parameters.repository_id),
                review_id: ReviewId::from(ids.review_id_parameters.review_id),
                text: String::from(&note_text),
                context: crate::worker::NoteContext::FileCodeLine {
                    file: FileDiffId::from(ids.file_diff_id),
                    line_content: String::from(line_content),
                    line_number: line_number as u32,
                },
            };
            channel.send(message).expect("Worker channel broken!");
        }
    });
    note_callbacks.on_set_file_line_note_context({
        let channel = worker_channel.clone();
        move |ids, file_id, line, content| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let note_id = NoteId::from(ids.note_id);
            let file_id = FileDiffId::from(file_id);
            let message = crate::worker::WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change: crate::worker::ReviewContent::Note {
                    note_id,
                    change_type: crate::worker::NoteChangeType::Context(crate::worker::NoteContext::FileCodeLine {
                        file: file_id,
                        line_content: String::from(&content),
                        line_number: line as u32,
                    }),
                },
            };
            channel.send(message).expect("Worker channel broken!");
        }
    });
    note_callbacks.on_copy_file_line_note({
        let channel = worker_channel.clone();
        let app_window_weak = app_window.as_weak();
        move |ids, file_diff_id, target_line| {
            let repository_id = RepositoryId::from(ids.review_id_parameters.repository_id);
            let review_id = ReviewId::from(ids.review_id_parameters.review_id);
            let note_id = NoteId::from(ids.note_id);
            let file_id = FileDiffId::from(file_diff_id);
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!");
            let note_model = model_utils::get_note_model(&app_window, repository_id.as_usize(), review_id.as_usize())
                .unwrap_or_else(|| panic!("[BUG] Could not query note model!"));
            let note_model = cast_model!(note_model, IdModel<ui::SlintNote>);
            let note = note_model
                .get(note_id.as_usize())
                .unwrap_or_else(|| panic!("Could not find note with id {}", note_id));

            let context = match note.context_type {
                ui::SlintContextType::FileLineNumber => note.file_line_context,
                _ => panic!("Selected note to copy, is not a file line note!"),
            };

            let message = crate::worker::WorkerMessage::AddNote {
                repository_id,
                review_id,
                text: String::from(&note.text),
                context: crate::worker::NoteContext::FileCodeLine {
                    file: file_id,
                    line_content: String::from(&context.line_content),
                    line_number: target_line as u32,
                },
            };
            channel.send(message).expect("Worker channel broken!");
        }
    });
}
