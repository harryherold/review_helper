use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use slint::{ComponentHandle, SharedString};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::repository_storage::{DiffRangeStore, ReviewName};
use crate::storage::{RepositoryName, RepositoryStore, create_storage};
use crate::ui::{SlintFileDiff, SlintNote};
use crate::{git_utils, ui};

use crate::repositories::{FileDiffId, NoteId, Repositories, RepositoryId, Review, ReviewId};
use crate::worker::{ReviewHelperSettings, ui_updater};

use crate::worker::ui_updater::{UiBasicRepository, UiUpdater};

pub type WorkerChannel = UnboundedSender<WorkerMessage>;

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    GitCommandFailed(String),
    NoGitDirectory(String),
}

#[derive(Clone)]
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
    FindFileDifferences {
        repository_id: RepositoryId,
        review_id: ReviewId,
        diff_range: DiffRangeStore,
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

    let ui_updater = UiUpdater::new(ui_weak.clone());

    let storage = create_storage(app_data_path);

    let mut repositories = Repositories::new(storage.load_repositories().expect("Could not load repositories!"));

    {
        let ui_repositories: Vec<_> = repositories
            .iter()
            .map(|(id, repo)| (id.as_i32(), UiBasicRepository::new(&repo.store())))
            .collect();

        ui_updater.initialize_repositories(ui_repositories);
    }

    ui_updater.initialize_review_helper_settings(&review_helper_settings);
    query_diff_tools(&ui_updater);

    while let Some(message) = rx.blocking_recv() {
        match message {
            WorkerMessage::Quit => return,
            WorkerMessage::QueryDiffTools => query_diff_tools(&ui_updater),
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
                    ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                }
            }
            WorkerMessage::NewRepository(path) => {
                if repositories.contains_repository_path(&path) {
                    ui_updater.report_error(ui::SlintResult::RepositoryExists, &path.to_string_lossy().as_ref());
                    continue;
                }
                match create_repository_store(path) {
                    Ok(store) => match storage.save_repository(&store) {
                        Ok(()) => {
                            let ui_repository = UiBasicRepository::new(&store);
                            let repository_id = repositories.add_repository(store);

                            ui_updater.new_repository(repository_id.as_i32(), ui_repository);
                        }
                        Err(e) => ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string()),
                    },
                    Err(e) => report_review_helper_error(&ui_updater, &e),
                }
            }
            WorkerMessage::ChangeRepository { id, base_branch } => {
                if let Some(repository) = repositories.get_mut(&id) {
                    repository.set_base_branch(base_branch);
                    if let Err(_) = storage.save_repository(&repository.store()) {
                        ui_updater.report_error(ui::SlintResult::StoreFailed, repository.name().as_str());
                    }
                } else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository  id {}", id.as_usize()));
                }
            }
            WorkerMessage::LoadReviewNames { id } => {
                if let Some(repository) = repositories.get_mut(&id) {
                    match storage.load_review_names(&repository.name()) {
                        Ok(review_names) => {
                            let mut reviews = Vec::new();
                            review_names.into_iter().for_each(|review_name| {
                                let id = repository.reviews.register_review_name(review_name.clone());
                                reviews.push((id.as_i32(), SharedString::from(review_name.as_str())));
                            });
                            ui_updater.set_review_names(id.as_usize(), reviews);
                        }
                        Err(e) => ui_updater.report_error(ui::SlintResult::LoadReviewNamesFailed, &e.to_string()),
                    }
                } else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", id.as_usize()));
                }
            }
            WorkerMessage::LoadReview { repository_id, review_id } => {
                if let Some(repository) = repositories.get_mut(&repository_id) {
                    let Some(review_name) = repository.reviews.review_name(&review_id) else {
                        ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("review_id {} not found!", review_id.as_usize()));
                        continue;
                    };
                    match storage.load_review(&repository.name(), review_name) {
                        Ok(opt_store) => {
                            if let Some(store) = opt_store {
                                let start_diff = SharedString::from(&store.diff_range.start);
                                let end_diff = SharedString::from(&store.diff_range.end);

                                let review = Review::new(store, review_name.clone());
                                let ui_notes: Vec<_> = review.notes.iter().map(|id_store_tuple| SlintNote::from(id_store_tuple)).collect();
                                let ui_file_diffs: Vec<_> = review.file_diffs.iter().map(|id_store_tuple| SlintFileDiff::from(id_store_tuple)).collect();

                                repository.reviews.insert_review(review_id.clone(), review);

                                ui_updater.set_review(repository_id.as_usize(), review_id.as_usize(), start_diff, end_diff, ui_notes, ui_file_diffs);
                            }
                        }
                        Err(e) => ui_updater.report_error(ui::SlintResult::LoadReviewFailed, &e.to_string()),
                    }
                } else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
                }
            }
            WorkerMessage::NewReview { repository_id, name } => {
                if let Some(repository) = repositories.get_mut(&repository_id) {
                    let review_name = ReviewName::from(name.as_str());
                    if repository.reviews.has_review_name(&review_name) {
                        ui_updater.report_error(ui::SlintResult::ReviewAlreadyExists, &name);
                        continue;
                    }
                    if let Err(e) = storage.save_review_file_diffs(repository.name(), &review_name, &DiffRangeStore::default(), &[]) {
                        ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &e.to_string());
                        continue;
                    }

                    let review_id = repository.reviews.new_review(review_name);
                    ui_updater.new_review(repository_id.as_usize(), review_id.as_usize(), SharedString::from(name.as_str()));
                } else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
                }
            }
            WorkerMessage::ChangeReview {
                repository_id,
                review_id,
                content_change,
            } => {
                let Some(repository) = repositories.get_mut(&repository_id) else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
                    continue;
                };

                let repository_name = repository.name().clone();

                match repository.reviews.get_mut(&review_id) {
                    Some(review) => match content_change {
                        ReviewContentChange::FileDiffChange { id: file_diff_id, is_reviewed } => {
                            review.file_diffs.set_is_reviewed(&file_diff_id, is_reviewed);
                            if let Err(e) = storage.save_review_file_diffs(&repository_name, &review.name(), &review.diff_range(), &review.file_diffs.stores())
                            {
                                ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                                continue;
                            }
                            ui_updater.set_file_diff_is_reviewed(repository_id.as_usize(), review_id.as_usize(), file_diff_id.as_usize(), is_reviewed);
                        }
                        ReviewContentChange::NoteChange { id, change_type } => {
                            let Some(note) = review.notes.get_mut(&id) else {
                                ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("note id {}", id.as_usize()));
                                continue;
                            };
                            match change_type.clone() {
                                NoteChangeType::TextChanged(new_text) => note.text = new_text,
                                NoteChangeType::ContextChanged(new_context) => note.context = new_context,
                                NoteChangeType::IsDoneChanged(new_is_done) => note.is_done = new_is_done,
                            }
                            if let Err(e) = storage.save_review_notes(&repository_name, &review.name(), &review.notes.stores()) {
                                ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                                continue;
                            }
                            ui_updater.update_note(repository_id.as_usize(), review_id.as_usize(), id.as_usize(), change_type);
                        }
                    },
                    None => ui_updater.report_error(
                        ui::SlintResult::ModelItemNotExists,
                        &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
                    ),
                }
            }
            WorkerMessage::FindFileDifferences {
                repository_id,
                review_id,
                diff_range,
            } => {
                let Some(repository) = repositories.get_mut(&repository_id) else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
                    continue;
                };
                let repository_name = repository.name().clone();

                let Ok(file_diff_map) = git_utils::diff_git_repo(&repository.path(), &diff_range.start, &diff_range.end) else {
                    ui_updater.report_error(ui::SlintResult::FindFileDifferenceFailed, &"".to_string());
                    continue;
                };

                let new_files = file_diff_map.keys().cloned().collect::<HashSet<_>>();

                let Some(review) = repository.reviews.get_mut(&review_id) else {
                    ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &format!("review id {}", review_id.as_usize()));
                    continue;
                };
                review.file_diffs.update_file_diffs(new_files);
                review.set_diff_range(diff_range);

                let ui_file_diffs = review
                    .file_diffs
                    .iter()
                    .map(|(id, store)| {
                        let file = store.file_path.to_string_lossy().to_string();
                        let diff_status = file_diff_map.get(&file).expect("Could not found Diff-Status of cached file!");
                        ui_updater::make_slint_file_diff(id, &file, &diff_status, store.is_reviewed)
                    })
                    .collect::<Vec<_>>();

                ui_updater.set_file_diffs(repository_id.as_usize(), review_id.as_usize(), ui_file_diffs);

                // TODO DRY!
                if let Err(e) = storage.save_review_file_diffs(&repository_name, &review.name(), review.diff_range(), &review.file_diffs.stores()) {
                    ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                }
            }
        }
    }
}

fn query_diff_tools(ui_updater: &UiUpdater) {
    let result = git_utils::query_diff_tools();
    match result {
        Err(e) => ui_updater.report_error(ui::SlintResult::QueryingDiffToolsFailed, &e.to_string()),
        Ok(diff_tools) => ui_updater.set_diff_tools(diff_tools),
    }
}

fn report_review_helper_error(ui_updater: &UiUpdater, error: &ReviewHelperError) {
    use ReviewHelperError::*;

    let (ui_error, ui_error_text) = match error {
        GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, t.as_str()),
        NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, t.as_str()),
    };
    ui_updater.report_error(ui_error, ui_error_text);
}
