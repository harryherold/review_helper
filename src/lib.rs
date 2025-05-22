use slint::Weak;
use std::{cell::RefCell, cmp::Ordering, env, ffi::OsStr, path::{Path, PathBuf}, process, rc::Rc, str::FromStr};
use anyhow::Result;
use chrono::{DateTime, FixedOffset};
use id_model::IdModel;
use project::Project;
use project_config::ProjectConfig;
use slint::{ComponentHandle, FilterModel, Model, ModelExt, ModelRc, SharedString, SortModel, StandardListViewItem, VecModel};

use native_dialog::FileDialog;
use ui::DiffFileItem;
use crate::command_utils::run_command;
use crate::id_model::IdModelChange;
use crate::ui::AppWindow;

mod app_config;
mod git_utils;
mod id_model;
mod notes;
mod project;
mod project_config;
mod repository;
mod command_utils;

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
    fn sort_by_extension(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
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
            self.sort_model = Rc::new(self.filter_model.clone().sort_by(Self::sort_by_extension));
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

type CommitFilterModel = Rc<FilterModel<ModelRc<ModelRc<StandardListViewItem>>, Box<dyn Fn(&ModelRc<StandardListViewItem>) -> bool>>>;
type CommitSortModel = Rc<SortModel<CommitFilterModel, Box<dyn Fn(&ModelRc<StandardListViewItem>, &ModelRc<StandardListViewItem>) -> Ordering>>>;
struct CommitProxyModel {
    filter_model: CommitFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    sort_model: CommitSortModel,
}

impl CommitProxyModel {
    fn get_sort_callback(sort_index: usize, is_sort_ascending: bool) -> Box<dyn Fn(&ModelRc<StandardListViewItem>, &ModelRc<StandardListViewItem>) -> Ordering> {
        Box::new(move |lhs: &ModelRc<StandardListViewItem>, rhs: &ModelRc<StandardListViewItem>| -> Ordering {
            let compare_string_columns = || -> Ordering {
                if is_sort_ascending {
                    lhs.row_data(sort_index).unwrap().text.cmp(&rhs.row_data(sort_index).unwrap().text)
                }
                else {
                    rhs.row_data(sort_index).unwrap().text.cmp(&lhs.row_data(sort_index).unwrap().text)
                }
            };
            let compare_date_columns = || -> Ordering {
                let lhs_date: DateTime<FixedOffset> = DateTime::from_str(&lhs.row_data(sort_index).unwrap().text).unwrap();
                let rhs_date: DateTime<FixedOffset> = DateTime::from_str(&rhs.row_data(sort_index).unwrap().text).unwrap();
                if is_sort_ascending {
                    lhs_date.cmp(&rhs_date)
                }
                else {
                    rhs_date.cmp(&lhs_date)
                }
            };
            if sort_index == 3 {
                compare_date_columns()
            }
            else {
                compare_string_columns()
            }
        })
    }

    fn new(model: ModelRc<ModelRc<StandardListViewItem>>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();

        let fm: CommitFilterModel = Rc::new(FilterModel::new(model, Box::new(move |row| {
            let filter_text = filter_text.clone();
            let pattern = filter_text.borrow();
            let message = row.row_data(1).unwrap();
            if pattern.is_empty() {
                return true;
            } else {
                message.text.to_lowercase().contains(&pattern.as_str().to_lowercase())
            }
        })));

        CommitProxyModel {
            filter_model: fm.clone(),
            filter_text: clone_filter_text,
            sort_model: Rc::new(fm.sort_by(CommitProxyModel::get_sort_callback(3, false))),
        }
    }

    fn sort_by(&mut self, sort_index: usize, is_sort_ascending: bool) {
        self.sort_model = Rc::new(self.filter_model.clone().sort_by(CommitProxyModel::get_sort_callback(sort_index, is_sort_ascending)));
    }
}

impl Default for CommitProxyModel {
    fn default() -> Self {
        let model: ModelRc<ModelRc<StandardListViewItem>> = Rc::new(VecModel::<ModelRc<StandardListViewItem>>::default()).into();
        let fm: CommitFilterModel = Rc::new(model.filter(Box::new(|_| true)));
        CommitProxyModel {
            filter_model: fm.clone(),
            filter_text: Rc::new(RefCell::new(SharedString::new())),
            sort_model: Rc::new(fm.sort_by(CommitProxyModel::get_sort_callback(3, false))),
        }
    }
}

fn parse_commandline_args() -> Option<PathBuf> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 3 && args[1] == "--project-file" {
        let path = PathBuf::from(args[2].clone());
        if !path.exists() {
            eprintln!("Given project file does not exist!");
            None
        }
        else {
            Some(PathBuf::from(args[2].clone()))
        }
    }
    else {
        None
    }
}

pub fn main() -> Result<(), slint::PlatformError> {
    let app_window = ui::AppWindow::new().unwrap();

    app_window.on_close(move || process::exit(0));

    let file_diff_model_ctx = Rc::new(RefCell::new(FileDiffModelContext::default()));
    let commit_proxy_model = Rc::new(RefCell::new(CommitProxyModel::default()));

    let project = setup_project(&app_window, file_diff_model_ctx.clone(), commit_proxy_model.clone());
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

    app_window.global::<ui::CommitPickerAdapter>().on_refresh({
        let project = project.clone();
        move || {
            let mut project = project.borrow_mut();
            project.repository.initialize_commits();
        }
    });
    app_window.global::<ui::CommitPickerAdapter>().on_filter_commits({
        let commit_proxy_model = commit_proxy_model.clone();
        move |pattern| {
            let m = commit_proxy_model.borrow_mut();
            *m.filter_text.borrow_mut() = pattern;
            m.filter_model.reset();
        }
    });

    app_window.global::<ui::CommitPickerAdapter>().on_sort_commits({
        let commit_proxy_model = commit_proxy_model.clone();
        let ui_weak = app_window.as_weak();
        move |sort_index, is_sort_ascending| {
            let ui = ui_weak.unwrap();
            let mut m = commit_proxy_model.borrow_mut();
            m.sort_by(sort_index as usize, is_sort_ascending);
            ui.global::<ui::CommitPickerAdapter>().set_commit_model(m.sort_model.clone().into());
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
    let app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
    let app_config = match app_config::AppConfig::new(app_data_path) {
        Ok(config) => Rc::new(RefCell::new(config)),
        Err(e) => {
            eprintln!("{}", e.to_string());
            Rc::new(RefCell::new(app_config::AppConfig::default()))
        }
    };

    app_window_handle.global::<ui::AppConfig>().on_save({
        let app_config = app_config.clone();
        let ui_weak = app_window_handle.as_weak();

        move || {
            let ui = ui_weak.unwrap();
            let mut app_config = app_config.borrow_mut();
            let ui_app_config = ui.global::<ui::AppConfig>();

            app_config.config.diff_tool = ui_app_config.get_diff_tool().to_string();
            app_config.config.editor = ui_app_config.get_editor().to_string();
            app_config.config.editor_args = ui_app_config.get_editor_args().split(",").map(|s| s.to_string()).collect();

            if let Err(e) = app_config.save() {
                eprintln!("Errors occurred during app config save: {}", e.to_string());
            }
        }
    });

    app_window_handle
        .global::<ui::AppConfig>()
        .set_diff_tool(SharedString::from(app_config.borrow().config.diff_tool.clone()));

    app_window_handle.global::<ui::AppConfig>().set_editor(SharedString::from(app_config.borrow().config.editor.clone()));

    let editor_args = app_config.borrow().config.editor_args.join(",");
    app_window_handle.global::<ui::AppConfig>().set_editor_args(SharedString::from(editor_args));

    app_config
}

fn modification_observer(ui_weak: Weak<AppWindow>) -> Box<dyn Fn(IdModelChange)> {
    let ui = ui_weak.clone().unwrap();
    Box::new(move |_: IdModelChange| { ui.global::<ui::Project>().set_has_modifications(true) })
}

fn setup_project(app_window_handle: &ui::AppWindow, file_diff_model_ctx: Rc<RefCell<FileDiffModelContext>>, commit_proxy_model: Rc<RefCell<CommitProxyModel>>) -> Rc<RefCell<Project>> {
    let read_project = |path| -> anyhow::Result<Project> {
        let project_config = ProjectConfig::read_from(&path)?;
        Project::from_config(&path, project_config)
    };
    let init_ui = |project: Rc<RefCell<Project>>, ui_weak: Weak<AppWindow>, file_diff_model_ctx: Rc<RefCell<FileDiffModelContext>>, commit_proxy_model: Rc<RefCell<CommitProxyModel>>| {
        let ui = ui_weak.unwrap();
        let project = project.borrow();

        ui.global::<ui::Project>().set_path(SharedString::from(project.path.to_str().unwrap()));
        if let Some(repo_path) = project.repository.repository_path() {
            ui.global::<ui::Repository>().set_path(SharedString::from(repo_path));
        }
        
        project.notes.observe_notes_model(modification_observer(ui_weak.clone()));
        
        ui.global::<ui::Notes>().set_notes_model(project.notes.notes_model().into());
        
        let (start_diff, end_diff) = project.repository.diff_range();
        ui.global::<ui::Diff>().set_start_commit(SharedString::from(start_diff));
        ui.global::<ui::Diff>().set_end_commit(SharedString::from(end_diff));
        
        let s = project.repository.statistics();
        ui.global::<ui::OverallDiffStats>().set_model(s.statistics_model.clone().into());
        
        project.repository.observe_file_diff_model(modification_observer(ui_weak.clone()));
        
        *file_diff_model_ctx.borrow_mut() = FileDiffModelContext::new(project.repository.file_diff_model());
        let m = file_diff_model_ctx.borrow();
        ui.global::<ui::Diff>().set_diff_model(m.sort_model.clone().into());

        let commit_proxy_model = commit_proxy_model.clone(); 
        *commit_proxy_model.borrow_mut() = CommitProxyModel::new(project.repository.commits_model());
        let p = commit_proxy_model.borrow();
        ui.global::<ui::CommitPickerAdapter>().set_commit_model(p.sort_model.clone().into());
    };

    let project = {
        match parse_commandline_args() {
            None => Rc::new(RefCell::new(Project::default())),
            Some(path) => {
                let project_result = read_project(path);
                if let Err(error) = project_result {
                    eprintln!("Could not read config: {}", error.to_string());
                    Rc::new(RefCell::new(Project::default()))
                }
                else {
                    Rc::new(RefCell::new(project_result.unwrap()))
                }
            }
        }
    };
    
    if project.borrow().path.exists() {
        init_ui(project.clone(), app_window_handle.as_weak(), file_diff_model_ctx.clone(), commit_proxy_model.clone());
    }
    
    app_window_handle.global::<ui::Project>().on_open({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        let commit_proxy_model = commit_proxy_model.clone();
        move || {
            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_open_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            if let Ok(new_project) = read_project(path_option.unwrap()) {
                *project_ref.borrow_mut() = new_project;
                init_ui(project_ref.clone(), ui_weak.clone(), file_diff_model_ctx.clone(), commit_proxy_model.clone());
            } else {
                eprintln!("Error occurred while loading config!");
            }
        }
    });
    app_window_handle.global::<ui::Project>().on_new({
        let ui_weak = app_window_handle.as_weak();
        let project_ref = project.clone();
        let file_diff_model_ctx = file_diff_model_ctx.clone();
        let commit_proxy_model = commit_proxy_model.clone();
        move || {
            let path_option = FileDialog::new().add_filter("toml project file", &["toml"]).show_save_single_file().unwrap();
            if path_option.is_none() {
                return;
            }
            if let Ok(new_project) = Project::new(&path_option.unwrap()) {
                *project_ref.borrow_mut() = new_project;
                init_ui(project_ref.clone(), ui_weak.clone(), file_diff_model_ctx.clone(), commit_proxy_model.clone());
            } else {
                eprintln!("Error occurred while loading config!");
            }
        }
    });
    app_window_handle.global::<ui::Project>().on_save({
        let project_ref = project.clone();
        let ui = app_window_handle.as_weak().unwrap();
        move || {
            if let Err(error) = project_ref.borrow().save() {
                eprintln!("Error occurred while saving: {}", error.to_string())
            }
            else {
                ui.global::<ui::Project>().set_has_modifications(false)
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
                    if let Some(old_path) = project_ref.repository.repository_path() {
                        if old_path == repo_path.to_str().expect("Could not convert path to string!") {
                            return;
                        }
                    }
                    ui.global::<ui::Project>().set_has_modifications(true);
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
            let (old_start_commit, old_end_commit) = {
                let project = project_ref.borrow();
                let (old_start, old_end) = project.repository.diff_range();
                (SharedString::from(old_start), SharedString::from(old_end))
            };
            let result = project_ref.borrow_mut().repository.diff_repository(&start_commit, &end_commit);
            if let Err(error) = result {
                eprintln!("Error on diffing repo: {}", error.to_string());
                return;
            }

            let ui = ui_weak.unwrap();
            if old_start_commit != start_commit || old_end_commit != end_commit {
                ui.global::<ui::Project>().set_has_modifications(true);
            }
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
            if let Err(error) = project_ref.borrow().repository.diff_file(id, &app_config.borrow().config.diff_tool) {
                eprintln!("Error occurred while file diff: {}", error.to_string())
            }
        }
    });
    app_window_handle.global::<ui::Diff>().on_open_file({
        let project_ref = project.clone();
        let app_config = app_config.clone();
        move |file_path| {
            let project = project_ref.borrow();
            let repo_path = project.repository.repository_path().expect("Repository path is not set!");
            let app_config = app_config.borrow();
            let args = app_config.config.editor_args.iter().map(|arg    | {
                if arg.contains("{file}") {
                    arg.replace("{file}", file_path.as_str())
                }
                else {
                    arg.to_string()
                }
            }).collect::<Vec<String>>();
            if let Err(error) = run_command(&app_config.config.editor, &args, &PathBuf::from(repo_path)) {
                eprintln!("Error occurred while opening file: {}", error.to_string())
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
