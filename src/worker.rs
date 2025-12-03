use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelExt, SharedString, VecModel};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::{RepositoryName, RepositoryStore, create_storage};
use crate::{git_utils, ui};

use crate::model::{IdModel, ReviewHelperCache, ReviewHelperError, ReviewHelperSettings};

pub type WorkerChannel = UnboundedSender<WorkerMessage>;

pub enum WorkerMessage {
    Quit,
    QueryDiffTools,
    SaveReviewHelperSettings {
        diff_tool: String,
        editor: String,
        editor_args: Vec<String>,
        color_scheme: String,
    },
    NewRepository(PathBuf),
    ChangeRepository {
        name: RepositoryName,
        base_branch: String,
    },
}

pub struct Worker {
    pub channel: UnboundedSender<WorkerMessage>,
    join_handle: std::thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(app_window: &ui::AppWindow) -> Self {
        let (channel, rx) = tokio::sync::mpsc::unbounded_channel();
        let worker_thread = std::thread::spawn({
            let ui_handle = app_window.as_weak();
            move || {
                worker_loop(ui_handle, rx);
            }
        });
        Self {
            channel,
            join_handle: worker_thread,
        }
    }
    pub fn join(self) -> std::thread::Result<()> {
        let _ = self.channel.send(WorkerMessage::Quit);
        self.join_handle.join()
    }
}

fn prepare_app_data_path() -> PathBuf {
    let mut app_data_path = dirs::data_local_dir().expect("Could not find OS specific dirs!");
    app_data_path.push(std::env!("CARGO_CRATE_NAME"));
    if !app_data_path.exists() {
        let result = fs::create_dir(&app_data_path);
        assert!(result.is_ok());
    }
    app_data_path
}

fn create_repository_store(path: PathBuf) -> Result<RepositoryStore, ReviewHelperError> {
    let path_str = path.to_str().unwrap_or_default();

    if !git_utils::is_git_repo(&path) {
        return Err(ReviewHelperError::NoGitDirectory(path_str.to_string()));
    }

    let name = path.file_name().unwrap_or_default().to_str().unwrap_or_default();
    let first_commit = git_utils::first_commit(&path)
        .map_err(|e| ReviewHelperError::GitCommandFailed(e.to_string()))?
        .into();

    let repository_name = RepositoryName::from(name);

    let repository_store = RepositoryStore {
        base_branch: "main".to_string(),
        path: path,
        first_commit,
        name: repository_name,
    };

    Ok(repository_store)
}

fn worker_loop(ui_weak: slint::Weak<ui::AppWindow>, mut rx: UnboundedReceiver<WorkerMessage>) {
    let app_data_path = prepare_app_data_path();
    let mut review_helper_settings = match ReviewHelperSettings::new(&app_data_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", e.to_string());
            ReviewHelperSettings::default()
        }
    };
    let mut review_helper_cache = ReviewHelperCache::default();
    let storage = create_storage(app_data_path);

    match storage.load_repositories() {
        Ok(repositories) => {
            review_helper_cache.set_repositories(&repositories);
            initialize_ui_repositories(ui_weak.clone(), repositories);
        }
        Err(_) => {
            panic!("Could not load repositories!")
        }
    }

    initialize_ui_review_helper_settings(ui_weak.clone(), &review_helper_settings);

    while let Some(message) = rx.blocking_recv() {
        match message {
            WorkerMessage::Quit => return,
            WorkerMessage::QueryDiffTools => query_diff_tools(ui_weak.clone()),
            WorkerMessage::SaveReviewHelperSettings {
                diff_tool,
                editor,
                editor_args,
                color_scheme,
            } => {
                review_helper_settings.diff_tool = diff_tool;
                review_helper_settings.editor = editor;
                review_helper_settings.editor_args = editor_args;
                review_helper_settings.color_scheme = color_scheme;
                if let Err(e) = review_helper_settings.save() {
                    report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, &e.to_string());
                }
            }
            WorkerMessage::NewRepository(path) => {
                if review_helper_cache.contains_repository_path(&path) {
                    report_error(ui_weak.clone(), ui::SlintResult::RepositoryExists, &path.to_string_lossy().as_ref());
                    continue;
                }
                match create_repository_store(path) {
                    Ok(store) => match storage.save_repository(&store) {
                        Ok(()) => {
                            review_helper_cache.add_repository(store.clone());
                            new_ui_repository(ui_weak.clone(), store);
                        }
                        Err(e) => report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, &e.to_string()),
                    },
                    Err(e) => report_review_helper_error(ui_weak.clone(), &e),
                }
            }
            WorkerMessage::ChangeRepository { name, base_branch } => {
                if let Some(store) = review_helper_cache.get_mut_repository_store(&name) {
                    store.base_branch = base_branch;
                    if let Err(_) = storage.save_repository(store) {
                        report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, name.as_str());
                    }
                } else {
                    report_error(ui_weak.clone(), ui::SlintResult::ModelItemNotExists, &name.as_str());
                }
            }
        }
    }
}

fn new_ui_repository(ui_weak: slint::Weak<ui::AppWindow>, store: RepositoryStore) {
    ui_weak
        .upgrade_in_event_loop({
            move |app_window| {
                let model_rc = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                let model = model_rc.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                let id = model.row_count();
                model.add(id, ui::SlintRepository::from((id, &store)));
            }
        })
        .unwrap();
}

fn initialize_ui_repositories(ui_weak: slint::Weak<ui::AppWindow>, repositories: Vec<RepositoryStore>) {
    ui_weak
        .upgrade_in_event_loop({
            move |app_window| {
                let model = IdModel::default();

                repositories
                    .iter()
                    .enumerate()
                    .for_each(|(id, store)| model.add(id, ui::SlintRepository::from((id, store))));

                let model_rc = Rc::new(model);
                let respository_name_model = model_rc.clone().map(|repository| repository.name);

                app_window
                    .global::<ui::SlintReviewHelper>()
                    .set_repository_names(Rc::new(respository_name_model).into());

                app_window.global::<ui::SlintReviewHelper>().set_repositories(model_rc.into());

                app_window.global::<ui::SlintErrors>().set_model(Rc::new(VecModel::default()).into());
            }
        })
        .unwrap();
}

fn initialize_ui_review_helper_settings(ui_weak: slint::Weak<ui::AppWindow>, review_helper_settings: &ReviewHelperSettings) {
    ui_weak
        .upgrade_in_event_loop({
            let diff_tool = SharedString::from(&review_helper_settings.diff_tool);
            let editor = SharedString::from(&review_helper_settings.editor);
            let editor_args = SharedString::from(&review_helper_settings.editor_args.join(","));
            let color_scheme = SharedString::from(&review_helper_settings.color_scheme);

            move |app_window| {
                app_window
                    .global::<ui::SlintReviewHelperSettings>()
                    .set_diff_tool(SharedString::from(diff_tool));
                app_window.global::<ui::SlintReviewHelperSettings>().set_editor(editor);
                app_window.global::<ui::SlintReviewHelperSettings>().set_editor_args(editor_args);
                app_window.global::<ui::SlintReviewHelperSettings>().set_color_scheme(color_scheme.clone());
                app_window.set_config_color_scheme(color_scheme);
            }
        })
        .unwrap();

    query_diff_tools(ui_weak);
}

fn query_diff_tools(ui_weak: slint::Weak<ui::AppWindow>) {
    let result = git_utils::query_diff_tools();
    match result {
        Err(e) => report_error(ui_weak.clone(), ui::SlintResult::QueryingDiffToolsFailed, &e.to_string()),
        Ok(diff_tools) => {
            ui_weak
                .upgrade_in_event_loop({
                    let ui_diff_tools: Vec<SharedString> = diff_tools.iter().map(|t| SharedString::from(t)).collect();
                    move |app_window| {
                        let model: VecModel<_> = VecModel::from(ui_diff_tools);
                        app_window.global::<ui::SlintReviewHelperSettings>().set_diff_tool_model(Rc::new(model).into());
                    }
                })
                .unwrap();
        }
    }
}

fn report_error(ui_weak: slint::Weak<ui::AppWindow>, error: ui::SlintResult, detail_text: &str) {
    let detail_text = SharedString::from(detail_text);
    ui_weak
        .upgrade_in_event_loop(move |app_window| {
            let model_rc = app_window.global::<ui::SlintErrors>().get_model();
            let model = model_rc.as_any().downcast_ref::<VecModel<ui::SlintErrorEntry>>().unwrap();
            model.push(ui::SlintErrorEntry {
                error_type: error.clone(),
                text: detail_text,
            });
            app_window.invoke_request_show_error(error);
        })
        .unwrap();
}

fn report_review_helper_error(ui_weak: slint::Weak<ui::AppWindow>, error: &ReviewHelperError) {
    use crate::model::ReviewHelperError::*;

    let (ui_error, ui_error_text) = match error {
        GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, t.as_str()),
        NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, t.as_str()),
        RepositoryExists(t) => (ui::SlintResult::RepositoryExists, t.as_str()),
        StoreFailed(t) => (ui::SlintResult::StoreFailed, t.as_str()),
    };
    report_error(ui_weak, ui_error, ui_error_text);
}
