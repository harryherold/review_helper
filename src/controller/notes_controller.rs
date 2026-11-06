use std::rc::Rc;

use slint::language::{DragAction, DropEvent};
use slint::{ComponentHandle, DataTransfer, Model, ModelRc};

use crate::model::{IdModel, model_utils};
use crate::repositories::{FileDiffId, RepositoryId, ReviewId};
use crate::worker::WorkerChannel;
use crate::{cast_model, ui, unwrap_or_return};

#[derive(Clone)]
struct DragPayload {
    line_no: i32,
}

pub fn setup_notes_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::Dapi>().on_make_data({
        move |line_no| {
            let mut transfer = DataTransfer::default();
            transfer.set_user_data(Rc::new(DragPayload { line_no }));
            transfer
        }
    });
    app_window.global::<ui::Dapi>().on_can_drop({
        move |event: DropEvent, target_line_no: i32| -> DragAction {
            if event.data.user_data().and_then(|rc| rc.downcast::<DragPayload>().ok()).is_some() {
                return event.proposed_action;
            }
            DragAction::None
        }
    });
    app_window.global::<ui::SlintNoteCallbacks>().on_note_source_model({
        let app_window_weak = app_window.as_weak();
        move |ids| -> ModelRc<ui::SlintNote> {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", ModelRc::default());
            let review = model_utils::get_slint_review(&app_window, ids.repository_id as usize, ids.review_id as usize);
            let review = review.unwrap_or_else(|| panic!("[BUG] {}, {} not found", ids.repository_id, ids.review_id));
            review.note_model
        }
    });
    app_window.global::<ui::SlintNoteCallbacks>().on_index_of_id({
        move |id, note_model| -> i32 {
            let note_model = cast_model!(note_model, IdModel<ui::SlintNote>);
            note_model.id_to_index(id as usize).map(|i| i as i32).unwrap_or(-1)
        }
    });
    app_window.global::<ui::SlintNoteCallbacks>().on_add_code_line_note({
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
}
