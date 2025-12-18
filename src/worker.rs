use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelExt, SharedString, VecModel};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::repository_storage::{DiffRangeStore, FileDiffStore, NoteStore, ReviewName};
use crate::storage::{RepositoryName, RepositoryStore, create_storage};
use crate::ui::{SlintFileDiff, SlintNote, SlintReview};
use crate::{git_utils, ui};

use crate::model::{IdModel, ReviewHelperSettings};
use crate::review_helper_cache::{FileDiffId, NoteId, RepositoryId, Review, ReviewHelperCache, ReviewHelperError, ReviewId};

pub type WorkerChannel = UnboundedSender<WorkerMessage>;

pub enum NoteChangeType {
    TextChanged(String),
    ContextChanged(String),
    IsDoneChanged(bool),
}

pub enum ReviewContentChange {
    NoteChange { id: NoteId, change_type: NoteChangeType },
    FileDiffChange { id: FileDiffId, is_reviewed: bool },
}

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
        id: RepositoryId,
        base_branch: String,
    },
    LoadReviewNames {
        id: RepositoryId,
    },
    LoadReview {
        repository_id: RepositoryId,
        review_id: ReviewId,
    },
    NewReview {
        repository_id: RepositoryId,
        name: String,
    },
    ChangeReview {
        repository_id: RepositoryId,
        review_id: ReviewId,
        content_change: ReviewContentChange,
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

impl From<(&NoteId, &NoteStore)> for ui::SlintNote {
    fn from((id, note_store): (&NoteId, &NoteStore)) -> Self {
        SlintNote {
            id: id.as_i32(),
            context: SharedString::from(note_store.context.as_str()),
            is_fixed: note_store.is_done,
            text: SharedString::from(note_store.text.as_str()),
        }
    }
}

impl From<(&FileDiffId, &FileDiffStore)> for ui::SlintFileDiff {
    fn from((id, file_diff_store): (&FileDiffId, &FileDiffStore)) -> Self {
        SlintFileDiff {
            id: id.as_i32(),
            is_reviewed: file_diff_store.is_reviewed,
            text: SharedString::from(file_diff_store.file_path.to_string_lossy().as_ref()),
            ..Default::default()
        }
    }
}

struct UiBasicRepository {
    path: SharedString,
    name: SharedString,
    first_commit: SharedString,
    base_branch: SharedString,
}

impl UiBasicRepository {
    fn new(repository_store: &RepositoryStore) -> Self {
        UiBasicRepository {
            first_commit: SharedString::from(repository_store.first_commit.as_str()),
            name: SharedString::from(repository_store.name.as_str()),
            path: SharedString::from(repository_store.path.to_string_lossy().as_ref()),
            base_branch: SharedString::from(repository_store.base_branch.as_str()),
        }
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
            let ui_repositories: Vec<_> = review_helper_cache
                .repositories
                .iter()
                .map(|(id, repo)| (id.as_i32(), UiBasicRepository::new(&repo.store)))
                .collect();
            initialize_ui_repositories(ui_weak.clone(), ui_repositories);
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
                            let ui_repository = UiBasicRepository::new(&store);
                            let repository_id = review_helper_cache.add_repository(store);

                            new_ui_repository(ui_weak.clone(), repository_id.as_i32(), ui_repository);
                        }
                        Err(e) => report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, &e.to_string()),
                    },
                    Err(e) => report_review_helper_error(ui_weak.clone(), &e),
                }
            }
            WorkerMessage::ChangeRepository { id, base_branch } => {
                if let Some(repository) = review_helper_cache.repositories.get_mut(&id) {
                    repository.store.base_branch = base_branch;
                    if let Err(_) = storage.save_repository(&repository.store) {
                        report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, repository.name.as_str());
                    }
                } else {
                    report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository  id {}", id.as_usize()),
                    );
                }
            }
            WorkerMessage::LoadReviewNames { id } => {
                if let Some(repository) = review_helper_cache.repositories.get_mut(&id) {
                    match storage.load_review_names(&repository.name) {
                        Ok(review_names) => {
                            let mut reviews = Vec::new();
                            review_names.into_iter().for_each(|review_name| {
                                let id = repository.register_review_name(review_name.clone());
                                reviews.push((id.as_i32(), SharedString::from(review_name.as_str())));
                            });
                            set_ui_review_names(ui_weak.clone(), id.as_usize(), reviews);
                        }
                        Err(e) => report_error(ui_weak.clone(), ui::SlintResult::LoadReviewNamesFailed, &e.to_string()),
                    }
                } else {
                    report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {}", id.as_usize()),
                    );
                }
            }
            WorkerMessage::LoadReview { repository_id, review_id } => {
                if let Some(repository) = review_helper_cache.repositories.get_mut(&repository_id) {
                    let Some(review_name) = repository.get_review_name(&review_id) else {
                        report_error(
                            ui_weak.clone(),
                            ui::SlintResult::ModelItemNotExists,
                            &format!("review_id {} not found!", review_id.as_usize()),
                        );
                        continue;
                    };
                    match storage.load_review(&repository.name, review_name) {
                        Ok(opt_store) => {
                            if let Some(store) = opt_store {
                                let start_diff = SharedString::from(&store.diff_range.start);
                                let end_diff = SharedString::from(&store.diff_range.end);

                                let review = Review::new(store, review_name.clone());
                                let ui_notes: Vec<_> = review.notes.iter().map(|id_store_tuple| SlintNote::from(id_store_tuple)).collect();
                                let ui_file_diffs: Vec<_> = review.file_diffs.iter().map(|id_store_tuple| SlintFileDiff::from(id_store_tuple)).collect();

                                repository.insert_review(review_id.clone(), review);

                                set_ui_review(
                                    ui_weak.clone(),
                                    repository_id.as_usize(),
                                    review_id.as_usize(),
                                    start_diff,
                                    end_diff,
                                    ui_notes,
                                    ui_file_diffs,
                                );
                            }
                        }
                        Err(e) => report_error(ui_weak.clone(), ui::SlintResult::LoadReviewFailed, &e.to_string()),
                    }
                } else {
                    report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {}", repository_id.as_usize()),
                    );
                }
            }
            WorkerMessage::NewReview { repository_id, name } => {
                if let Some(repository) = review_helper_cache.repositories.get_mut(&repository_id) {
                    let review_name = ReviewName::from(name.as_str());
                    if repository.has_review_name(&review_name) {
                        report_error(ui_weak.clone(), ui::SlintResult::ReviewAlreadyExists, &name);
                        continue;
                    }
                    if let Err(e) = storage.save_review_file_diffs(&repository.name, &review_name, &DiffRangeStore::default(), &[]) {
                        report_error(ui_weak.clone(), ui::SlintResult::ModelItemNotExists, &e.to_string());
                        continue;
                    }

                    let review_id = repository.new_review(review_name);
                    new_ui_review(
                        ui_weak.clone(),
                        repository_id.as_usize(),
                        review_id.as_usize(),
                        SharedString::from(name.as_str()),
                    );
                } else {
                    report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {}", repository_id.as_usize()),
                    );
                }
            }
            WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            } => {
                let Some(repository) = review_helper_cache.repositories.get_mut(&repository_id) else {
                    report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {}", repository_id.as_usize()),
                    );
                    continue;
                };

                let repository_name = repository.name.clone();

                match repository.get_mut_review(&review_id) {
                    Some(review) => match content_change {
                        ReviewContentChange::FileDiffChange { id, is_reviewed } => {
                            let Some(file_diff) = review.file_diffs.get_mut(&id) else {
                                report_error(ui_weak.clone(), ui::SlintResult::ModelItemNotExists, &format!("file diff id {}", id.as_usize()));
                                continue;
                            };
                            file_diff.is_reviewed = is_reviewed;
                            if let Err(e) = storage.save_review_file_diffs(
                                &repository_name,
                                &review.name,
                                &review.diff_range,
                                &review.file_diffs.values().collect::<Vec<_>>(),
                            ) {
                                report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, &e.to_string());
                            }
                        }
                        ReviewContentChange::NoteChange { id, change_type } => {
                            let Some(note) = review.notes.get_mut(&id) else {
                                report_error(ui_weak.clone(), ui::SlintResult::ModelItemNotExists, &format!("note id {}", id.as_usize()));
                                continue;
                            };
                            match change_type {
                                NoteChangeType::TextChanged(new_text) => note.text = new_text,
                                NoteChangeType::ContextChanged(new_context) => note.context = new_context,
                                NoteChangeType::IsDoneChanged(new_is_done) => note.is_done = new_is_done,
                            }
                            if let Err(e) = storage.save_review_notes(&repository_name, &review.name, &review.notes.values().collect::<Vec<_>>()) {
                                report_error(ui_weak.clone(), ui::SlintResult::StoreFailed, &e.to_string());
                            }
                        }
                    },
                    None => report_error(
                        ui_weak.clone(),
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
                    ),
                }
            }
        }
    }
}

fn new_ui_review(ui_weak: slint::Weak<ui::AppWindow>, repository_id: usize, review_id: usize, review_name: SharedString) {
    ui_weak
        .upgrade_in_event_loop(move |app_window| {
            let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
            let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
            let repository = repository_model.get(repository_id).expect("Repository model is out of sync with cache!");
            let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();

            assert!(false == review_model.has(review_id));

            review_model.add(
                review_id,
                SlintReview {
                    id: review_id as i32,
                    name: review_name,
                    is_loaded: true,
                    ..Default::default()
                },
            );
        })
        .unwrap();
}

fn set_ui_review(
    ui_weak: slint::Weak<ui::AppWindow>,
    repository_id: usize,
    review_id: usize,
    start_diff: SharedString,
    end_diff: SharedString,
    ui_notes: Vec<SlintNote>,
    ui_file_diffs: Vec<SlintFileDiff>,
) {
    ui_weak
        .upgrade_in_event_loop(move |app_window| {
            let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
            let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
            let repository = repository_model.get(repository_id).expect("Repository model is out of sync with cache!");
            let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
            let mut review = review_model.get(review_id).expect("Review model is out of sync with cache");
            review.start_diff = start_diff;
            review.end_diff = end_diff;
            review.is_loaded = true;

            let notes_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
            ui_notes.into_iter().for_each(|ui_note| notes_model.add(ui_note.id as usize, ui_note));

            let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
            ui_file_diffs
                .into_iter()
                .for_each(|ui_file_diff| file_diff_model.add(ui_file_diff.id as usize, ui_file_diff));

            review_model.update(review_id, review);
        })
        .unwrap();
}

fn set_ui_review_names(ui_weak: slint::Weak<ui::AppWindow>, repository_id: usize, reviews: Vec<(i32, SharedString)>) {
    ui_weak
        .upgrade_in_event_loop(move |app_window| {
            let model_rc = app_window.global::<ui::SlintReviewHelper>().get_repositories();
            let model = model_rc.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
            let repository = model.get(repository_id).unwrap();
            let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
            reviews.into_iter().for_each(|(id, name)| {
                review_model.add(
                    id as usize,
                    ui::SlintReview {
                        id,
                        name: name.clone(),
                        note_model: Rc::new(IdModel::default()).into(),
                        file_diff_model: Rc::new(IdModel::default()).into(),
                        is_loaded: false,
                        ..Default::default()
                    },
                );
            });
        })
        .unwrap();
}

fn new_ui_repository(ui_weak: slint::Weak<ui::AppWindow>, repository_id: i32, ui_repository: UiBasicRepository) {
    ui_weak
        .upgrade_in_event_loop({
            move |app_window| {
                let model_rc = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                let model = model_rc.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                model.add(
                    repository_id as usize,
                    ui::SlintRepository {
                        id: repository_id,
                        base_branch: ui_repository.base_branch,
                        first_commit: ui_repository.first_commit,
                        name: ui_repository.name,
                        path: ui_repository.path,
                        review_model: Rc::new(IdModel::default()).into(),
                    },
                );
            }
        })
        .unwrap();
}

fn initialize_ui_repositories(ui_weak: slint::Weak<ui::AppWindow>, repositories: Vec<(i32, UiBasicRepository)>) {
    ui_weak
        .upgrade_in_event_loop({
            move |app_window| {
                let model = IdModel::default();

                repositories.into_iter().for_each(|(id, ui_repository)| {
                    model.add(
                        id as usize,
                        ui::SlintRepository {
                            id,
                            base_branch: ui_repository.base_branch,
                            first_commit: ui_repository.first_commit,
                            name: ui_repository.name,
                            path: ui_repository.path,
                            review_model: Rc::new(IdModel::default()).into(),
                        },
                    );
                });

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
    use ReviewHelperError::*;

    let (ui_error, ui_error_text) = match error {
        GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, t.as_str()),
        NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, t.as_str()),
    };
    report_error(ui_weak, ui_error, ui_error_text);
}
