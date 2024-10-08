use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use slint::ComponentHandle;

mod notes;
mod repository;

use notes::Notes;
use repository::Repository;

pub mod ui;

pub fn main() -> Result<(), slint::PlatformError> {
    let app_window = ui::AppWindow::new().unwrap();

    let _repository = setup_repository(&app_window);
    let _notes = setup_notes(&app_window);

    app_window.run()
}

fn setup_repository(app_window_handle: &ui::AppWindow) -> Rc<RefCell<Repository>> {
    let repository = Rc::new(RefCell::new(Repository::new()));
    app_window_handle.global::<ui::Repository>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let r = repository.clone();
        move || {
            let ui = ui_weak.unwrap();
            let path = r.borrow_mut().open();
            ui.global::<ui::Repository>().set_path(path);
        }
    });
    app_window_handle.global::<ui::Diff>().on_diff_start_end({
        let ui_weak = app_window_handle.as_weak();
        let repo = repository.clone();
        move |start_commit, end_commit| {
            repo.borrow_mut().diff_repository(&start_commit, &end_commit);

            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_start_commit(start_commit);
            ui.global::<ui::Diff>().set_end_commit(end_commit);
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file_diff({
        let repo = repository.clone();
        move |index| repo.borrow().diff_file(index)
    });
    app_window_handle
        .global::<ui::Diff>()
        .set_diff_model(repository.borrow().file_diff_model().into());

    repository
}

fn setup_notes(app_window_handle: &ui::AppWindow) -> Rc<RefCell<Notes>> {
    let notes = Rc::new(RefCell::new(Notes::new()));
    app_window_handle.global::<ui::Notes>().on_open({
        let n = notes.clone();
        let ui_weak = app_window_handle.as_weak();
        move || {
            if let Some(path) = n.borrow_mut().open() {
                let ui = ui_weak.unwrap();
                ui.global::<ui::Notes>().set_path(path);
            }
        }
    });
    app_window_handle.global::<ui::Notes>().on_save({
        let n = notes.clone();
        let ui_weak = app_window_handle.as_weak();
        move || {
            if let Some(path) = n.borrow_mut().save() {
                let ui = ui_weak.unwrap();
                ui.global::<ui::Notes>().set_path(path);
            }
        }
    });
    app_window_handle.global::<ui::Notes>().on_add_note({
        let n = notes.clone();
        move |text| n.borrow().add_note(text)
    });
    app_window_handle.global::<ui::Notes>().on_change_text({
        let n = notes.clone();
        move |todo_index, text| n.borrow().set_note_text(todo_index as usize, text)
    });
    app_window_handle.global::<ui::Notes>().on_toggle_fixed({
        let n = notes.clone();
        move |todo_index| n.borrow().toogle_is_fixed(todo_index as usize)
    });
    app_window_handle.global::<ui::Notes>().set_notes_model(notes.borrow().notes_model().into());
    notes
}
