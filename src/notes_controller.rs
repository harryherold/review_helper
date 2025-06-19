use std::rc::Rc;

use slint::{ComponentHandle, ModelExt};

use crate::app_state::AppState;
use crate::ui;

pub fn setup_notes(app_state: &AppState) {
    app_state.app_window.global::<ui::Notes>().on_add_note({
        let project_ref = app_state.project.clone();
        move |text, context| project_ref.borrow_mut().notes.add_note(text, context)
    });
    app_state.app_window.global::<ui::Notes>().on_change_text({
        let project_ref = app_state.project.clone();
        move |id, text| project_ref.borrow_mut().notes.set_note_text(id as usize, text)
    });
    app_state.app_window.global::<ui::Notes>().on_toggle_fixed({
        let project_ref = app_state.project.clone();
        move |id| project_ref.borrow_mut().notes.toggle_is_fixed(id as usize)
    });
    app_state.app_window.global::<ui::Notes>().on_file_notes_model({
        let project_ref = app_state.project.clone();
        let models = app_state.notes_proxy_models.borrow().filtered_file_proxy_models.clone();
        move |file| {
            let file_string = file.to_string();
            let mut models = models.borrow_mut();
            if models.contains_key(&file.to_string()) {
                models.get(&file.to_string()).unwrap().clone()
            } else {
                let notes = project_ref.borrow_mut().notes.notes_model();
                let file_notes = Rc::new(notes.clone().filter(move |item: &ui::NoteItem| item.context.contains(file.as_str())));
                models.insert(file_string, file_notes.clone().into());
                file_notes.into()
            }
        }
    });
    app_state.app_window.global::<ui::Notes>().on_delete_note({
        let project_ref = app_state.project.clone();
        move |id| project_ref.borrow_mut().notes.delete_note(id as usize)
    });
    app_state.app_window.global::<ui::Notes>().on_change_context({
        let project_ref = app_state.project.clone();
        move |id, context| {
            project_ref.borrow().notes.change_context(id as usize, context);
        }
    });
    app_state.app_window.global::<ui::Notes>().on_set_notes_text_filter({
        let notes_proxy_models = app_state.notes_proxy_models.clone();
        move |pattern| {
            notes_proxy_models.borrow().set_text_filter(pattern);
        }
    });
    app_state.app_window.global::<ui::Notes>().on_set_notes_context_filter({
        let notes_proxy_models = app_state.notes_proxy_models.clone();
        move |pattern| {
            notes_proxy_models.borrow().set_context_filter(pattern);
        }
    });
}
