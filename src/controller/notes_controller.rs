use slint::{ComponentHandle, Model, ModelRc};

use crate::model::{IdModel, model_utils};
use crate::{cast_model, ui, unwrap_or_return};

pub fn setup_notes_callbacks(app_window: &ui::AppWindow) {
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
}
