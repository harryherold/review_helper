use std::{
    cell::RefCell,
    cmp::Ordering,
    ffi::OsStr,
    path::{Path, PathBuf},
    process,
    rc::Rc,
};

use anyhow::Result;

use id_model::IdModel;
use project::Project;
use project_config::ProjectConfig;
use slint::{ComponentHandle, FilterModel, ModelExt, ModelRc, SharedString, SortModel};

use native_dialog::FileDialog;
use ui::DiffFileItem;

mod app_config;
mod git_utils;
mod id_model;
mod notes;
mod project;
mod project_config;
mod repository;

pub mod ui;

type FileDiffFilterModel = Rc<FilterModel<ModelRc<DiffFileItem>, Box<dyn Fn(&ui::DiffFileItem) -> bool>>>;
type FileDiffSortModel = Rc<SortModel<FileDiffFilterModel, fn(&ui::DiffFileItem, &ui::DiffFileItem) -> Ordering>>;

struct FileDiffModelContext {
    filter_model: FileDiffFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    sort_model: FileDiffSortModel,
}

impl FileDiffModelContext {
    fn sort_by_name(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
        lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
    }
    fn sort_by_exentsion(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
        let lhs_opt = extension_from_filename(&lhs.text);
        let rhs_opt = extension_from_filename(&rhs.text);
        if lhs_opt.is_some() && rhs_opt.is_some() {
            let result = lhs_opt.unwrap().cmp(rhs_opt.unwrap());
            if result == Ordering::Equal {
                lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
            } else {
                result
            }
        } else if lhs_opt.is_some() && rhs_opt.is_none() {
            Ordering::Greater
        } else if lhs_opt.is_none() && rhs_opt.is_some() {
            Ordering::Less
        } else {
            lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
        }
    }

    fn new(model: ModelRc<ui::DiffFileItem>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();

        let fm: FileDiffFilterModel = Rc::new(FilterModel::new(
            model,
            Box::new(move |item: &ui::DiffFileItem| -> bool {
                let filter_text = filter_text.clone();
                let pattern = filter_text.borrow();
                if pattern.is_empty() {
                    return true;
                } else {
                    item.text.to_lowercase().contains(&pattern.as_str().to_lowercase())
                }
            }),
        ));

        FileDiffModelContext {
            filter_model: fm.clone(),
            filter_text: clone_filter_text,
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }

    fn sort_by(&mut self, sort_criteria: ui::SortCriteria) {
        if sort_criteria == ui::SortCriteria::Name {
            self.sort_model = Rc::new(self.filter_model.clone().sort_by(Self::sort_by_name));
        } else {
            self.sort_model = Rc::new(self.filter_model.clone().sort_by(Self::sort_by_exentsion));
        }
    }
}

impl Default for FileDiffModelContext {
    fn default() -> Self {
        let model: ModelRc<ui::DiffFileItem> = Rc::new(IdModel::<ui::DiffFileItem>::default()).into();
        let fm: FileDiffFilterModel = Rc::new(model.filter(Box::new(|_| true)));
        FileDiffModelContext {
            filter_model: fm.clone(),
            filter_text: Rc::new(RefCell::new(SharedString::new())),
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }
}

pub fn main() -> Result<(), slint::PlatformError> {
    let app_window = ui::AppWindow::new().unwrap();

    app_window.on_close(move || process::exit(0));

    let file_diff_model_ctx = Rc::new(RefCell::new(FileDiffModelContext::default()));
    let project = setup_project(&app_window, file_diff_model_ctx.clone());
    let app_config = setup_app_config(&app_window);

    setup_repository(&app_window, &project, &app_config, file_diff_model_ctx.clone());
    setup_notes(&app_window, &project);

    app_window.global::<ui::Diff>().on_filter_file_diff({
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        move |pattern| {
            let m = file_diff_model_ctx.borrow_mut();
            *m.filter_text.borrow_mut() = pattern;
            m.filter_model.reset();
        }
    });

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

fn extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

fn setup_app_config(app_window_handle: &ui::AppWindow) -> Rc<RefCell<app_config::AppConfig>> {
    let app_config = match app_config::AppConfig::new(app_config::config_dir_path()) {
        Ok(config) => Rc::new(RefCell::new(config)),
        Err(e) => {
            eprintln!("{}", e.to_string());
            Rc::new(RefCell::new(app_config::AppConfig::default()))
        }
    };

    app_window_handle.global::<ui::AppConfig>().on_change_diff_tool({
        let ui_weak = app_window_handle.as_weak();
        let app_config = app_config.clone();
        move |diff_tool| {
            let ui = ui_weak.unwrap();
            app_config.borrow_mut().set_diff_tool(diff_tool.to_string());
            ui.global::<ui::AppConfig>().set_diff_tool(diff_tool);
        }
    });
    app_window_handle.global::<ui::AppConfig>().on_save({
        let app_config = app_config.clone();
        move || {
            if let Err(e) = app_config.borrow().save() {
                eprintln!("Errors occurred during app config save: {}", e.to_string());
            }
        }
    });

    app_window_handle
        .global::<ui::AppConfig>()
        .set_diff_tool(SharedString::from(app_config.borrow().diff_tool()));

    app_config
}

fn setup_project(app_window_handle: &ui::AppWindow, file_diff_model_ctx: Rc<RefCell<FileDiffModelContext>>) -> Rc<RefCell<Project>> {
    let project = Rc::new(RefCell::new(Project::default()));

    app_window_handle.global::<ui::Project>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        move || {
            let ui = ui_weak.unwrap();

            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_open_single_file().unwrap();

            if path_option.is_none() {
                return;
            }
            let path = path_option.unwrap();
            let config_result = ProjectConfig::read_from(&path);
            if let Err(error) = config_result {
                eprintln!("Could not read config: {}", error.to_string());
                return;
            }
            let project_config = config_result.unwrap();
            if let Ok(new_project) = Project::from_config(&path, project_config) {
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

                let s = project.repository.statistics();
                ui.global::<ui::OverallDiffStats>().set_model(s.statistics_model.clone().into());

                *file_diff_model_ctx.borrow_mut() = FileDiffModelContext::new(project.repository.file_diff_model());
                let m = file_diff_model_ctx.borrow();
                ui.global::<ui::Diff>().set_diff_model(m.sort_model.clone().into());
            } else {
                eprintln!("Error occured while loading config!");
            }
        }
    });
    app_window_handle.global::<ui::Project>().on_new({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        move || {
            let ui = ui_weak.unwrap();
            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_save_single_file().unwrap();
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

                let s = project.repository.statistics();
                ui.global::<ui::OverallDiffStats>().set_model(s.statistics_model.clone().into());

                *file_diff_model_ctx.borrow_mut() = FileDiffModelContext::new(project.repository.file_diff_model());
                let m = file_diff_model_ctx.borrow();
                ui.global::<ui::Diff>().set_diff_model(m.sort_model.clone().into());
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

fn setup_repository(
    app_window_handle: &ui::AppWindow,
    project: &Rc<RefCell<Project>>,
    app_config: &Rc<RefCell<app_config::AppConfig>>,
    file_diff_model_ctx: Rc<RefCell<FileDiffModelContext>>,
) {
    app_window_handle.global::<ui::Repository>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        move || {
            let ui = ui_weak.unwrap();
            let mut project_ref = project_ref.borrow_mut();
            match FileDialog::new().set_location("~").show_open_single_dir().unwrap() {
                Some(repo_path) => {
                    if let Some(path) = repo_path.to_str() {
                        ui.global::<ui::Repository>().set_path(SharedString::from(path));
                    }
                    project_ref.repository.set_path(repo_path);
                }
                None => {}
            }
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
            let project = project_ref.borrow();
            let statistics = project.repository.statistics();

            ui.global::<ui::OverallDiffStats>().set_added_lines(statistics.added_lines as i32);
            ui.global::<ui::OverallDiffStats>().set_removed_lines(statistics.removed_lines as i32);
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file_diff({
        let project_ref = project.clone();
        let app_config = app_config.clone();
        move |id| {
            if let Err(error) = project_ref.borrow().repository.diff_file(id, &app_config.borrow().diff_tool()) {
                eprintln!("Error occured while file diff: {}", error.to_string())
            }
        }
    });
    app_window_handle.global::<ui::Diff>().on_toggle_is_reviewed({
        let project_ref = project.clone();
        move |id| project_ref.borrow_mut().repository.toggle_file_is_reviewed(id as usize)
    });
    app_window_handle.global::<ui::Diff>().on_set_sort_criteria({
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        let ui_weak = app_window_handle.as_weak();
        move |sort_criteria| {
            let ui = ui_weak.unwrap();
            ui.global::<ui::Diff>().set_current_sort_criteria(sort_criteria);
            file_diff_model_ctx.borrow_mut().sort_by(sort_criteria);
            let m = file_diff_model_ctx.borrow();
            ui.global::<ui::Diff>().set_diff_model(m.sort_model.clone().into());
        }
    });
}

fn setup_notes(app_window_handle: &ui::AppWindow, project: &Rc<RefCell<Project>>) {
    app_window_handle.global::<ui::Notes>().on_add_note({
        let project_ref = project.clone();
        move |text, context| project_ref.borrow_mut().notes.add_note(text, context)
    });
    app_window_handle.global::<ui::Notes>().on_change_text({
        let project_ref = project.clone();
        move |id, text| project_ref.borrow_mut().notes.set_note_text(id as usize, text)
    });
    app_window_handle.global::<ui::Notes>().on_toggle_fixed({
        let project_ref = project.clone();
        move |id| project_ref.borrow_mut().notes.toogle_is_fixed(id as usize)
    });
    app_window_handle.global::<ui::Notes>().on_file_notes_model({
        let project_ref = project.clone();
        move |file| {
            let notes = project_ref.borrow_mut().notes.notes_model();
            let file_notes = notes.clone().filter(move |item| item.context.contains(file.as_str()));
            Rc::new(file_notes).into()
        }
    });
    app_window_handle.global::<ui::Notes>().on_delete_note({
        let project_ref = project.clone();
        move |id| project_ref.borrow_mut().notes.delete_note(id as usize)
    });
}
