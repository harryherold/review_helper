use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use project::Project;
use slint::{ComponentHandle, SharedString};

use native_dialog::FileDialog;

mod notes;
mod project;
mod repository;

pub mod ui;

pub fn main() -> Result<(), slint::PlatformError> {
    let app_window = ui::AppWindow::new().unwrap();

    let project = setup_project(&app_window);
    setup_repository(&app_window, &project);
    setup_notes(&app_window, &project);

    app_window.run()
}

fn setup_project(app_window_handle: &ui::AppWindow) -> Rc<RefCell<Project>> {
    let project = Rc::new(RefCell::new(Project::new()));

    // app_window_handle.global::<ui::Project>().on_open({
    //     let ui_weak = app_window_handle.as_weak();
    //     let p = project.clone();
    //     move || {
    //         let ui = ui_weak.unwrap();
    //         let mut proj = p.borrow_mut();
    //         if let Ok(path) = proj.open() {
    //             ui.global::<ui::Project>().set_path(SharedString::from(path));
    //         } else {
    //             println!("Error occured while loading config!");
    //         }
    //     }
    // });

    app_window_handle.global::<ui::Project>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let p = project.clone();
        move || {
            let ui = ui_weak.unwrap();

            let path_option = FileDialog::new().add_filter("Ini project file", &["ini"]).show_open_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            let path = path_option.unwrap();

            if let Ok(new_p) = Project::open2(path.clone()) {
                *p.borrow_mut() = new_p;
                ui.global::<ui::Project>().set_path(SharedString::from(path.to_str().unwrap()));
            } else {
                println!("Error occured while loading config!");
            }

            // let mut proj = p.borrow_mut();
        }
    });

    project
}

fn setup_repository(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    app_window_handle.global::<ui::Repository>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let p = project.clone();
        move || {
            let ui = ui_weak.unwrap();
            let mut proj = p.borrow_mut();
            let path = proj.repository.open();
            ui.global::<ui::Repository>().set_path(SharedString::from(path));
        }
    });
    app_window_handle.global::<ui::Diff>().on_diff_start_end({
        let ui_weak = app_window_handle.as_weak();
        let p = project.clone();
        move |start_commit, end_commit| {
            let mut proj = p.borrow_mut();
            proj.repository.diff_repository(&start_commit, &end_commit);

            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_start_commit(start_commit);
            ui.global::<ui::Diff>().set_end_commit(end_commit);
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file_diff({
        let p = project.clone();
        move |index| p.borrow().repository.diff_file(index)
    });

    app_window_handle
        .global::<ui::Diff>()
        .set_diff_model(project.borrow().repository.file_diff_model().into());
}

fn setup_notes(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    // app_window_handle.global::<ui::Notes>().on_open({
    //     let n = notes.clone();
    //     let ui_weak = app_window_handle.as_weak();
    //     move || {
    //         if let Some(path) = n.borrow_mut().open() {
    //             let ui = ui_weak.unwrap();
    //             ui.global::<ui::Notes>().set_path(path);
    //         }
    //     }
    // });
    // app_window_handle.global::<ui::Notes>().on_save({
    //     let n = notes.clone();
    //     let ui_weak = app_window_handle.as_weak();
    //     move || {
    //         if let Some(path) = n.borrow_mut().save() {
    //             let ui = ui_weak.unwrap();
    //             ui.global::<ui::Notes>().set_path(path);
    //         }
    //     }
    // });
    app_window_handle.global::<ui::Notes>().on_add_note({
        let p = project.clone();
        move |text| p.borrow_mut().notes.add_note(text)
    });
    app_window_handle.global::<ui::Notes>().on_change_text({
        let p = project.clone();
        move |todo_index, text| p.borrow_mut().notes.set_note_text(todo_index as usize, text)
    });
    app_window_handle.global::<ui::Notes>().on_toggle_fixed({
        let p = project.clone();
        move |todo_index| p.borrow_mut().notes.toogle_is_fixed(todo_index as usize)
    });
    app_window_handle
        .global::<ui::Notes>()
        .set_notes_model(project.borrow().notes.notes_model().into());
}
