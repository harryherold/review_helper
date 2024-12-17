use std::{cell::RefCell, path::PathBuf, process, rc::Rc};

use anyhow::Result;

use config::Config;
use project::Project;
use slint::{ComponentHandle, ModelExt, SharedString};

use native_dialog::FileDialog;

mod config;
mod git_utils;
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

    app_window.global::<ui::StringUtils>().on_filename({
        |path| {
            if let Some(file_name) = PathBuf::from(path.to_string()).file_name() {
                file_name.to_str().expect("Could not parse os string!").to_string().into()
            } else {
                "".into()
            }
        }
    });

    app_window.run()
}

fn setup_project(app_window_handle: &ui::AppWindow) -> Rc<RefCell<Project>> {
    let project = Rc::new(RefCell::new(Project::default()));

    app_window_handle.global::<ui::Project>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move || {
            let ui = ui_weak.unwrap();

            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_open_single_file().unwrap();
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
            if let Ok(new_project) = Project::from_config(&path, config) {
                *project_ref.borrow_mut() = new_project;
                let project = project_ref.borrow();

                ui.global::<ui::Project>().set_path(SharedString::from(path.to_str().unwrap()));
                if let Some(repo_path) = project.repository.repository_path() {
                    ui.global::<ui::Repository>().set_path(SharedString::from(repo_path));
                }
                ui.global::<ui::Notes>().set_notes_model(project.notes.notes_model().into());

                let (start_diff, end_diff) = project.repository.diff_range();
                ui.global::<ui::Diff>().set_start_commit(SharedString::from(start_diff));
                ui.global::<ui::Diff>().set_end_commit(SharedString::from(end_diff));
                ui.global::<ui::Diff>().set_diff_model(project.repository.file_diff_model().into());
                let file_model = project.repository.file_diff_model().clone().map(|f| slint::format!("{}", f.text));
                ui.global::<ui::Diff>().set_file_model(Rc::new(file_model).into());
            } else {
                eprintln!("Error occured while loading config!");
            }
        }
    });
    app_window_handle.global::<ui::Project>().on_new({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move || {
            let ui = ui_weak.unwrap();
            let path_option = FileDialog::new().add_filter("Ini project file", &["ini"]).show_save_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            let path = path_option.unwrap();

            if let Ok(new_project) = Project::new(&path) {
                *project_ref.borrow_mut() = new_project;
                let project = project_ref.borrow();

                ui.global::<ui::Project>().set_path(SharedString::from(path.to_str().unwrap()));
                ui.global::<ui::Repository>().set_path("".into());
                ui.global::<ui::Notes>().set_notes_model(project.notes.notes_model().into());

                ui.global::<ui::Diff>().set_start_commit("".into());
                ui.global::<ui::Diff>().set_end_commit("".into());
                ui.global::<ui::Diff>().set_diff_model(project.repository.file_diff_model().into());
                let file_model = project.repository.file_diff_model().clone().map(|f| slint::format!("{}", f.text));
                ui.global::<ui::Diff>().set_file_model(Rc::new(file_model).into());
            } else {
                eprintln!("Error occured while loading config!");
            }
        }
    });
    app_window_handle.global::<ui::Project>().on_save({
        let project_ref = project.clone();
        move || {
            if let Err(error) = project_ref.borrow().save() {
                eprintln!("Error occured while saving: {}", error.to_string())
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
            let result = project_ref.borrow_mut().repository.diff_repository(&start_commit, &end_commit);
            if let Err(error) = result {
                eprintln!("Error on diffing repo: {}", error.to_string());
                return;
            }
            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_start_commit(start_commit);
            ui.global::<ui::Diff>().set_end_commit(end_commit);
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file_diff({
        let project_ref = project.clone();
        move |index| {
            if let Err(error) = project_ref.borrow().repository.diff_file(index) {
                eprintln!("Error occured while file diff: {}", error.to_string())
            }
        }
    });
    app_window_handle.global::<ui::Diff>().on_toggle_is_reviewed({
        let project_ref = project.clone();
        move |index| project_ref.borrow_mut().repository.toggle_file_is_reviewed(index as usize)
    });
}

fn setup_notes(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    app_window_handle.global::<ui::Notes>().on_add_note({
        let project_ref = project.clone();
        move |text, context| project_ref.borrow_mut().notes.add_note(text, context)
    });
    app_window_handle.global::<ui::Notes>().on_change_text({
        let project_ref = project.clone();
        move |todo_index, text| project_ref.borrow_mut().notes.set_note_text(todo_index, text)
    });
    app_window_handle.global::<ui::Notes>().on_toggle_fixed({
        let project_ref = project.clone();
        move |todo_index| project_ref.borrow_mut().notes.toogle_is_fixed(todo_index)
    });
    app_window_handle.global::<ui::Notes>().on_file_notes_model({
        let project_ref = project.clone();
        move |file| {
            let notes = project_ref.borrow_mut().notes.notes_model();
            let file_notes = notes.clone().filter(move |item| item.context.contains(file.as_str()));
            Rc::new(file_notes).into()
        }
    });
}
