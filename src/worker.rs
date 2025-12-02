use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelExt, SharedString, VecModel};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::ReviewHelperFileStorage;
use crate::{git_utils, ui};

use crate::model::{IdModel, ReviewHelper, ReviewHelperSettings};

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

fn worker_loop(ui_weak: slint::Weak<ui::AppWindow>, mut rx: UnboundedReceiver<WorkerMessage>) {
    let app_data_path = prepare_app_data_path();
    let mut review_helper_settings = match ReviewHelperSettings::new(&app_data_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", e.to_string());
            ReviewHelperSettings::default()
        }
    };
    let storage = ReviewHelperFileStorage::new(app_data_path);
    let mut review_helper = ReviewHelper::new(Rc::new(storage));

    initialize_repositories_ui(ui_weak.clone(), &review_helper);
    initialize_review_helper_settings_ui(ui_weak.clone(), &review_helper_settings);

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
        }
    }
}

fn initialize_repositories_ui(ui_weak: slint::Weak<ui::AppWindow>, review_helper: &ReviewHelper) {
    ui_weak
        .upgrade_in_event_loop({
            let repositories = review_helper.repository_stores.clone();
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

fn initialize_review_helper_settings_ui(ui_weak: slint::Weak<ui::AppWindow>, review_helper_settings: &ReviewHelperSettings) {
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
