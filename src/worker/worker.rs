use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use slint::{ComponentHandle, SharedString};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::repository_storage::{DiffRangeStore, ReviewName};
use crate::storage::{RepositoryName, RepositoryStore, ReviewHelperStorage, create_storage};
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
    DeleteRepository(RepositoryId),
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
    DeleteReview {
        repository_id: RepositoryId,
        review_id: ReviewId,
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
    ShowFileDifferences {
        repository_id: RepositoryId,
        review_id: ReviewId,
        file_diff_id: FileDiffId,
    },
    DeleteNote {
        repository_id: RepositoryId,
        review_id: ReviewId,
        note_id: NoteId,
    },
    AddNote {
        repository_id: RepositoryId,
        review_id: ReviewId,
        text: String,
        context: String,
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
                let mut worker_impl = WorkerImpl::new(ui_handle);
                worker_impl.worker_loop(rx);
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

struct WorkerImpl {
    ui_updater: UiUpdater,
    settings: ReviewHelperSettings,
    storage: Box<dyn ReviewHelperStorage>,
    repositories: Repositories,
}

impl WorkerImpl {
    fn new(ui_weak: slint::Weak<ui::AppWindow>) -> Self {
        let app_data_path = prepare_app_data_path();
        let review_helper_settings = match ReviewHelperSettings::new(&app_data_path) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("{}", e.to_string());
                ReviewHelperSettings::default()
            }
        };

        let ui_updater = UiUpdater::new(ui_weak);

        let storage = create_storage(app_data_path);

        let repositories = Repositories::new(storage.load_repositories().expect("Could not load repositories!"));

        {
            let ui_repositories: Vec<_> = repositories
                .iter()
                .map(|(id, repo)| (id.as_i32(), UiBasicRepository::new(&repo.store())))
                .collect();

            ui_updater.initialize_repositories(ui_repositories);
        }

        ui_updater.initialize_review_helper_settings(&review_helper_settings);

        let worker_impl = Self {
            ui_updater,
            settings: review_helper_settings,
            storage,
            repositories,
        };
        worker_impl.query_diff_tools();

        worker_impl
    }

    fn query_diff_tools(&self) {
        let result = git_utils::query_diff_tools();
        match result {
            Err(e) => self.ui_updater.report_error(ui::SlintResult::QueryingDiffToolsFailed, &e.to_string()),
            Ok(diff_tools) => self.ui_updater.set_diff_tools(diff_tools),
        }
    }
    fn report_review_helper_error(&self, error: &ReviewHelperError) {
        use ReviewHelperError::*;

        let (ui_error, ui_error_text) = match error {
            GitCommandFailed(t) => (ui::SlintResult::GitCommandFailed, t.as_str()),
            NoGitDirectory(t) => (ui::SlintResult::NoGitDirectory, t.as_str()),
        };
        self.ui_updater.report_error(ui_error, ui_error_text);
    }
    fn show_file_differences(&self, repository_id: RepositoryId, review_id: ReviewId, file_diff_id: FileDiffId) {
        let Some(repository) = self.repositories.get(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository  id {}", repository_id.as_usize()));
            return;
        };
        let Some(review) = repository.reviews.get(&review_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
            );
            return;
        };
        let Some(file_diff) = review.file_diffs.get(&file_diff_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!(
                    "repository id {} review id {} file diff id {}",
                    repository_id.as_usize(),
                    review_id.as_usize(),
                    file_diff_id.as_usize()
                ),
            );
            return;
        };
        let start_commit = review.diff_range().start.as_str();
        let end_commit = review.diff_range().end.as_str();
        let file = &file_diff.file_path.to_string_lossy();
        let file = file.as_ref();
        let diff_tool = self.settings.diff_tool.as_str();

        if let Err(e) = git_utils::diff_file(repository.path(), start_commit, end_commit, file, diff_tool) {
            self.ui_updater.report_error(ui::SlintResult::ShowFileDifferencesFailed, &e.to_string());
        }
    }
    fn worker_loop(&mut self, mut rx: UnboundedReceiver<WorkerMessage>) {
        while let Some(message) = rx.blocking_recv() {
            match message {
                WorkerMessage::Quit => return,
                WorkerMessage::QueryDiffTools => self.query_diff_tools(),
                WorkerMessage::SaveReviewHelperSettings {
                    diff_tool,
                    editor,
                    editor_args,
                    color_scheme,
                } => self.save_settings(diff_tool, editor, editor_args, color_scheme),
                WorkerMessage::NewRepository(path) => self.new_repository(path),
                WorkerMessage::ChangeRepository { id, base_branch } => self.change_repository(id, base_branch),
                WorkerMessage::LoadReviewNames { id } => self.load_review_names(id),
                WorkerMessage::LoadReview { repository_id, review_id } => self.load_review(repository_id, review_id),
                WorkerMessage::NewReview { repository_id, name } => self.new_review(repository_id, name),
                WorkerMessage::DeleteReview { repository_id, review_id } => self.delete_review(repository_id, review_id),
                WorkerMessage::ChangeReview {
                    repository_id,
                    review_id,
                    content_change,
                } => self.change_review(repository_id, review_id, content_change),
                WorkerMessage::FindFileDifferences {
                    repository_id,
                    review_id,
                    diff_range,
                } => self.find_file_difference(repository_id, review_id, diff_range),
                WorkerMessage::ShowFileDifferences {
                    repository_id,
                    review_id,
                    file_diff_id,
                } => self.show_file_differences(repository_id, review_id, file_diff_id),
                WorkerMessage::DeleteNote {
                    repository_id,
                    review_id,
                    note_id,
                } => self.delete_note(repository_id, review_id, note_id),
                WorkerMessage::AddNote {
                    repository_id,
                    review_id,
                    text,
                    context,
                } => self.add_note(repository_id, review_id, text, context),
                WorkerMessage::DeleteRepository(repository_id) => self.delete_repository(repository_id),
            }
        }
    }
    fn save_settings(&mut self, diff_tool: String, editor: String, editor_args: Vec<String>, color_scheme: String) {
        self.settings.diff_tool = diff_tool;
        self.settings.editor = editor;
        self.settings.editor_args = editor_args;
        self.settings.color_scheme = color_scheme;
        if let Err(e) = self.settings.save() {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
        }
    }
    fn new_repository(&mut self, path: PathBuf) {
        if self.repositories.contains_repository_path(&path) {
            self.ui_updater
                .report_error(ui::SlintResult::RepositoryExists, &path.to_string_lossy().as_ref());
            return;
        }
        match create_repository_store(path) {
            Ok(store) => match self.storage.save_repository(&store) {
                Ok(()) => {
                    let ui_repository = UiBasicRepository::new(&store);
                    let repository_id = self.repositories.add_repository(store);

                    self.ui_updater.new_repository(repository_id.as_i32(), ui_repository);
                }
                Err(e) => self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string()),
            },
            Err(e) => self.report_review_helper_error(&e),
        }
    }
    fn delete_repository(&mut self, repository_id: RepositoryId) {
        let Some(repository_name) = self.repositories.delete_repository(&repository_id) else {
            return;
        };
        if let Err(e) = self.storage.delete_repository(&repository_name) {
            self.ui_updater.report_error(ui::SlintResult::DeleteRepositoryFailed, &e.to_string());
            return;
        }
        self.ui_updater.delete_repository(repository_id.as_usize());
    }
    fn change_repository(&mut self, repository_id: RepositoryId, base_branch: String) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository  id {}", repository_id.as_usize()));
            return;
        };

        repository.set_base_branch(base_branch);
        if let Err(_) = self.storage.save_repository(&repository.store()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &repository.name.as_str());
        }
    }
    fn load_review_names(&mut self, repository_id: RepositoryId) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };

        match self.storage.load_review_names(&repository.name) {
            Ok(review_names) => {
                let mut reviews = Vec::new();
                review_names.into_iter().for_each(|review_name| {
                    let id = repository.reviews.register_review_name(review_name.clone());
                    reviews.push((id.as_i32(), SharedString::from(review_name.as_str())));
                });
                self.ui_updater.set_review_names(repository_id.as_usize(), reviews);
            }
            Err(e) => self.ui_updater.report_error(ui::SlintResult::LoadReviewNamesFailed, &e.to_string()),
        }
    }
    fn load_review(&mut self, repository_id: RepositoryId, review_id: ReviewId) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };

        let Some(review_name) = repository.reviews.review_name(&review_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("review_id {} not found!", review_id.as_usize()));
            return;
        };

        let load_result = self.storage.load_review(&repository.name, review_name);
        if let Err(e) = load_result {
            self.ui_updater.report_error(ui::SlintResult::LoadReviewFailed, &e.to_string());
            return;
        }

        let optional_store = load_result.unwrap_or_default();

        if let Some(store) = optional_store {
            let start_diff = SharedString::from(&store.diff_range.start);
            let end_diff = SharedString::from(&store.diff_range.end);

            let review = Review::new(store, review_name.clone());
            let ui_notes: Vec<_> = review.notes.iter().map(|id_store_tuple| SlintNote::from(id_store_tuple)).collect();
            let ui_file_diffs: Vec<_> = review.file_diffs.iter().map(|id_store_tuple| SlintFileDiff::from(id_store_tuple)).collect();

            repository.reviews.insert_review(review_id.clone(), review);

            self.ui_updater
                .set_review(repository_id.as_usize(), review_id.as_usize(), start_diff, end_diff, ui_notes, ui_file_diffs);
        }
    }
    fn new_review(&mut self, repository_id: RepositoryId, name: String) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };
        let review_name = ReviewName::from(name.as_str());
        if repository.reviews.has_review_name(&review_name) {
            self.ui_updater.report_error(ui::SlintResult::ReviewAlreadyExists, &name);
            return;
        }
        if let Err(e) = self
            .storage
            .save_review_file_diffs(&repository.name, &review_name, &DiffRangeStore::default(), &[])
        {
            self.ui_updater.report_error(ui::SlintResult::ModelItemNotExists, &e.to_string());
            return;
        }

        let review_id = repository.reviews.new_review(review_name);
        self.ui_updater
            .new_review(repository_id.as_usize(), review_id.as_usize(), SharedString::from(name.as_str()));
    }
    fn delete_review(&mut self, repository_id: RepositoryId, review_id: ReviewId) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };
        let Some(review_name) = repository.reviews.delete_review(&review_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
            );
            return;
        };

        if let Err(e) = self.storage.delete_review(&repository.name, &review_name) {
            self.ui_updater.report_error(ui::SlintResult::DeleteReviewFailed, &e.to_string());
            return;
        }

        self.ui_updater.delete_review(repository_id.as_usize(), review_id.as_usize());
    }
    fn change_review(&mut self, repository_id: RepositoryId, review_id: ReviewId, content_change: ReviewContentChange) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };

        let Some(review) = repository.reviews.get_mut(&review_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
            );
            return;
        };

        let change_file_diff = |review: &mut Review, file_diff_id: FileDiffId, is_reviewed: bool| {
            review.file_diffs.set_is_reviewed(&file_diff_id, is_reviewed);
            if let Err(e) = self
                .storage
                .save_review_file_diffs(&repository.name, &review.name(), &review.diff_range(), &review.file_diffs.stores())
            {
                self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                return;
            }
            self.ui_updater
                .set_file_diff_is_reviewed(repository_id.as_usize(), review_id.as_usize(), file_diff_id.as_usize(), is_reviewed);
        };
        let change_note = |review: &mut Review, note_id: NoteId, change_type: NoteChangeType| {
            let Some(note) = review.notes.get_mut(&note_id) else {
                self.ui_updater
                    .report_error(ui::SlintResult::ModelItemNotExists, &format!("note id {}", note_id.as_usize()));
                return;
            };
            match change_type.clone() {
                NoteChangeType::TextChanged(new_text) => note.text = new_text,
                NoteChangeType::ContextChanged(new_context) => note.context = new_context,
                NoteChangeType::IsDoneChanged(new_is_done) => note.is_done = new_is_done,
            }
            if let Err(e) = self.storage.save_review_notes(&repository.name, &review.name(), &review.notes.stores()) {
                self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                return;
            }
            self.ui_updater
                .update_note(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize(), change_type);
        };
        match content_change {
            ReviewContentChange::FileDiffChange { id, is_reviewed } => change_file_diff(review, id, is_reviewed),
            ReviewContentChange::NoteChange { id, change_type } => change_note(review, id, change_type),
        }
    }
    fn find_file_difference(&mut self, repository_id: RepositoryId, review_id: ReviewId, diff_range: DiffRangeStore) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };

        let Ok(file_diff_map) = git_utils::diff_git_repo(&repository.path(), &diff_range.start, &diff_range.end) else {
            self.ui_updater.report_error(ui::SlintResult::FindFileDifferenceFailed, &"".to_string());
            return;
        };

        let new_files = file_diff_map.keys().cloned().collect::<HashSet<_>>();

        let Some(review) = repository.reviews.get_mut(&review_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("review id {}", review_id.as_usize()));
            return;
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

        self.ui_updater.set_file_diffs(repository_id.as_usize(), review_id.as_usize(), ui_file_diffs);

        if let Err(e) = self
            .storage
            .save_review_file_diffs(&repository.name, &review.name(), review.diff_range(), &review.file_diffs.stores())
        {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
        }
    }
    fn delete_note(&mut self, repository_id: RepositoryId, review_id: ReviewId, note_id: NoteId) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };

        let Some(review) = repository.reviews.get_mut(&review_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
            );
            return;
        };

        if !review.notes.delete_note(&note_id) {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!(
                    "repository id {} review id {} note id {}",
                    repository_id.as_usize(),
                    review_id.as_usize(),
                    note_id.as_usize()
                ),
            );
            return;
        }

        if let Err(e) = self.storage.save_review_notes(&repository.name, &review.name(), &review.notes.stores()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }

        self.ui_updater.delete_note(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize());
    }
    fn add_note(&mut self, repository_id: RepositoryId, review_id: ReviewId, text: String, context: String) {
        let Some(repository) = self.repositories.get_mut(&repository_id) else {
            self.ui_updater
                .report_error(ui::SlintResult::ModelItemNotExists, &format!("repository id {}", repository_id.as_usize()));
            return;
        };
        let Some(review) = repository.reviews.get_mut(&review_id) else {
            self.ui_updater.report_error(
                ui::SlintResult::ModelItemNotExists,
                &format!("repository id {} review id {}", repository_id.as_usize(), review_id.as_usize()),
            );
            return;
        };

        let ui_text = SharedString::from(text.as_str());
        let ui_context = SharedString::from(context.as_str());

        let note_id = review.notes.add_note(text, context);

        if let Err(e) = self.storage.save_review_notes(&repository.name, &review.name(), &review.notes.stores()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }
        let ui_note = SlintNote {
            id: note_id.as_i32(),
            text: ui_text,
            context: ui_context,
            is_fixed: false,
        };
        self.ui_updater.add_note(repository_id.as_usize(), review_id.as_usize(), ui_note);
    }
}
