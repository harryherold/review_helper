use std::{cell::RefCell, process, rc::Rc};

use anyhow::Result;

use config::Config;
use project::Project;
use slint::{ComponentHandle, SharedString};

use native_dialog::FileDialog;

mod config;
mod notes;
mod project;
mod repository;

pub mod ui;

pub fn main() -> Result<(), slint::PlatformError> {
    let app_window = ui::AppWindow::new().unwrap();

    app_window.on_close(move || process::exit(0));
    let project = setup_project(&app_window);
    setup_repository(&app_window, &project);
    setup_notes(&app_window, &project);

    app_window.run()
}

fn setup_project(app_window_handle: &ui::AppWindow) -> Rc<RefCell<Project>> {
    let project = Rc::new(RefCell::new(Project::new()));

    app_window_handle.global::<ui::Project>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move || {
            let ui = ui_weak.unwrap();

            let path_option = FileDialog::new().add_filter("Ini project file", &["ini"]).show_open_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            let path = path_option.unwrap();
            let config_result = Config::read_from(&path);
            if let Err(error) = config_result {
                eprintln!("Could not read config: {}", error.to_string());
                return;
            }
            let config = config_result.unwrap();
            if let Ok(new_project) = Project::open(&config) {
                *project_ref.borrow_mut() = new_project;
                ui.global::<ui::Project>().set_path(SharedString::from(path.to_str().unwrap()));
                ui.global::<ui::Repository>()
                    .set_path(SharedString::from(project_ref.borrow().repository.repository_path()));
                ui.global::<ui::Notes>().set_notes_model(project_ref.borrow().notes.notes_model().into());

                ui.global::<ui::Diff>().set_start_commit(SharedString::from(config.start_diff));
                ui.global::<ui::Diff>().set_end_commit(SharedString::from(config.end_diff));
                ui.global::<ui::Diff>().set_diff_model(project_ref.borrow().repository.file_diff_model().into());
            } else {
                println!("Error occured while loading config!");
            }
        }
    });

    project
}

fn setup_repository(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    app_window_handle.global::<ui::Repository>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move || {
            let ui = ui_weak.unwrap();
            let mut project_ref = project_ref.borrow_mut();
            let path = project_ref.repository.open();
            ui.global::<ui::Repository>().set_path(SharedString::from(path));
        }
    });
    app_window_handle.global::<ui::Diff>().on_diff_start_end({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move |start_commit, end_commit| {
            project_ref.borrow_mut().repository.diff_repository(&start_commit, &end_commit);

            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_start_commit(start_commit);
            ui.global::<ui::Diff>().set_end_commit(end_commit);
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file_diff({
        let project_ref = project.clone();
        move |index| project_ref.borrow().repository.diff_file(index)
    });

    app_window_handle
        .global::<ui::Diff>()
        .set_diff_model(project.borrow().repository.file_diff_model().into());
}

fn setup_notes(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    app_window_handle.global::<ui::Notes>().on_add_note({
        let project_ref = project.clone();
        move |text| project_ref.borrow_mut().notes.add_note(text)
    });
    app_window_handle.global::<ui::Notes>().on_change_text({
        let project_ref = project.clone();
        move |todo_index, text| project_ref.borrow_mut().notes.set_note_text(todo_index as usize, text)
    });
    app_window_handle.global::<ui::Notes>().on_toggle_fixed({
        let project_ref = project.clone();
        move |todo_index| project_ref.borrow_mut().notes.toogle_is_fixed(todo_index as usize)
    });
    app_window_handle
        .global::<ui::Notes>()
        .set_notes_model(project.borrow().notes.notes_model().into());
}
