use crate::app_state::AppState;
use crate::commit_proxy_model::CommitProxyModel;
use crate::file_diff_proxy_models::FileDiffProxyModels;
use crate::files_proxy_model::FilesProxyModel;
use crate::id_model::IdModelChange;
use crate::notes_proxy_models::NotesProxyModels;
use crate::project::Project;
use crate::project_config::ProjectConfig;
use crate::ui;
use native_dialog::FileDialog;
use slint::{ComponentHandle, Model, SharedString, Weak};
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

pub fn setup_project(app_state: &mut AppState) {
    let read_project = |path| -> anyhow::Result<Project> {
        let project_config = ProjectConfig::read_from(&path)?;
        Project::from_config(&path, project_config)
    };
    let init_ui = |project: Rc<RefCell<Project>>,
                   ui_weak: Weak<ui::AppWindow>,
                   file_diff_model_ctx: Rc<RefCell<FileDiffProxyModels>>,
                   commit_proxy_model: Rc<RefCell<CommitProxyModel>>,
                   files_proxy_model: Rc<RefCell<FilesProxyModel>>,
                   notes_proxy_model: Rc<RefCell<NotesProxyModels>>,| {
        let ui = ui_weak.unwrap();
        let project = project.borrow();

        ui.global::<ui::Project>().set_path(SharedString::from(project.path.to_str().unwrap()));
        if let Some(repo_path) = project.repository.repository_path() {
            ui.global::<ui::Repository>().set_path(SharedString::from(repo_path));
        }

        project.notes.observe_notes_model(modification_observer(ui_weak.clone()));

        *notes_proxy_model.borrow_mut() = NotesProxyModels::new(project.notes.notes_model());
        let m = notes_proxy_model.borrow();
        ui.global::<ui::Notes>().set_notes_model(m.model());

        let (start_diff, end_diff) = project.repository.diff_range();
        ui.global::<ui::Diff>().set_start_commit(SharedString::from(start_diff));
        ui.global::<ui::Diff>().set_end_commit(SharedString::from(end_diff));

        let s = project.repository.statistics();
        ui.global::<ui::OverallDiffStats>().set_model(s.statistics_model.clone().into());

        project.repository.observe_file_diff_model(modification_observer(ui_weak.clone()));

        *file_diff_model_ctx.borrow_mut() = FileDiffProxyModels::new(project.repository.file_diff_model());
        let m = file_diff_model_ctx.borrow();
        ui.global::<ui::Diff>().set_diff_model(m.sort_model());

        let commit_proxy_model = commit_proxy_model.clone();
        *commit_proxy_model.borrow_mut() = CommitProxyModel::new(project.repository.commits_model());
        let p = commit_proxy_model.borrow();
        ui.global::<ui::CommitPickerAdapter>().set_commit_model(p.sort_model());

        let files_proxy_model = files_proxy_model.clone();
        *files_proxy_model.borrow_mut() = FilesProxyModel::new(project.repository.file_diff_model());
        let m = files_proxy_model.borrow();
        ui.global::<ui::FilePickerAdapter>().set_files_model(m.files_sort_model());
    };

    if let Some(path) = parse_commandline_args() {
        let project_result = read_project(path);
        if let Err(error) = project_result {
            eprintln!("Could not read config: {}", error.to_string());
        } else {
            app_state.project = Rc::new(RefCell::new(project_result.unwrap()));
            init_ui(
                app_state.project.clone(),
                app_state.app_window.as_weak(),
                app_state.file_diff_proxy_models.clone(),
                app_state.commit_proxy_model.clone(),
                app_state.files_proxy_model.clone(),
                app_state.notes_proxy_models.clone(),
            );
        }
    }

    app_state.app_window.global::<ui::Project>().on_open({
        let ui_weak = app_state.app_window.as_weak();
        let project_ref = app_state.project.clone();
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        let files_proxy_model = app_state.files_proxy_model.clone();
        let notes_proxy_model = app_state.notes_proxy_models.clone();
        move || {
            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_open_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            if let Ok(new_project) = read_project(path_option.unwrap()) {
                *project_ref.borrow_mut() = new_project;
                init_ui(
                    project_ref.clone(),
                    ui_weak.clone(),
                    file_diff_model_ctx.clone(),
                    commit_proxy_model.clone(),
                    files_proxy_model.clone(),
                    notes_proxy_model.clone(),
                );
            } else {
                eprintln!("Error occurred while loading config!");
            }
        }
    });

    app_state.app_window.global::<ui::Project>().on_new({
        let ui_weak = app_state.app_window.as_weak();
        let project_ref = app_state.project.clone();
        let file_diff_model_ctx = app_state.file_diff_proxy_models.clone();
        let commit_proxy_model = app_state.commit_proxy_model.clone();
        let files_proxy_model = app_state.files_proxy_model.clone();
        let notes_proxy_model = app_state.notes_proxy_models.clone();
        move || {
            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_save_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            if let Ok(new_project) = Project::new(&path_option.unwrap()) {
                *project_ref.borrow_mut() = new_project;
                init_ui(
                    project_ref.clone(),
                    ui_weak.clone(),
                    file_diff_model_ctx.clone(),
                    commit_proxy_model.clone(),
                    files_proxy_model.clone(),
                    notes_proxy_model.clone(),
                );
            } else {
                eprintln!("Error occurred while loading config!");
            }
        }
    });

    app_state.app_window.global::<ui::Project>().on_save({
        let project_ref = app_state.project.clone();
        let ui = app_state.app_window.as_weak().unwrap();
        move || {
            if let Err(error) = project_ref.borrow().save() {
                eprintln!("Error occurred while saving: {}", error.to_string())
            } else {
                ui.global::<ui::Project>().set_has_modifications(false)
            }
        }
    });

    app_state.app_window.global::<ui::FilePickerAdapter>().on_set_filter({
        let files_proxy_model = app_state.files_proxy_model.clone();
        move |pattern| {
            files_proxy_model.borrow_mut().set_filter_text(pattern);
        }
    });
    app_state.app_window.global::<ui::FilePickerAdapter>().on_contains_model_context({
        let files_proxy_model = app_state.files_proxy_model.clone();
        move |context| -> bool {
            let model = files_proxy_model.borrow().files_sort_model();
            model.iter().any(|file| file == context)
        }
    })
}

fn modification_observer(ui_weak: Weak<ui::AppWindow>) -> Box<dyn Fn(IdModelChange)> {
    let ui = ui_weak.clone().unwrap();
    Box::new(move |_: IdModelChange| ui.global::<ui::Project>().set_has_modifications(true))
}

fn parse_commandline_args() -> Option<PathBuf> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 && args[1] == "--project-file" {
        let path = PathBuf::from(args[2].clone());
        if !path.exists() {
            eprintln!("Given project file does not exist!");
            None
        } else {
            Some(PathBuf::from(args[2].clone()))
        }
    } else {
        None
    }
}
