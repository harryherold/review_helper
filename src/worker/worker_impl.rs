use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use slint::{ComponentHandle, SharedString};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::storage::repository_storage::{DiffRangeStore, ReviewName};
use crate::storage::{RepositoryName, RepositoryStore, ReviewHelperStorage, create_storage};
use crate::ui::{SlintContextType, SlintNote};
use crate::{git_utils, ui};

use crate::repositories::{FileDiffId, NoteId, Repositories, RepositoryId, Review, ReviewId};
use crate::worker::ReviewHelperSettings;

use crate::worker::ui_updater::{UiBasicRepository, UiUpdater};

pub type WorkerChannel = UnboundedSender<WorkerMessage>;

#[derive(Debug, Clone)]
pub enum ReviewHelperError {
    GitCommandFailed(String),
    NoGitDirectory(String),
}

#[derive(Clone)]
pub enum NoteChangeType {
    Text(String),
    Context(String),
    IsDone(bool),
}

pub enum ReviewContent {
    Note { note_id: NoteId, change_type: NoteChangeType },
    FileDiff { file_diff_id: FileDiffId, is_reviewed: bool },
    Name(ReviewName),
}

pub enum WorkerMessage {
    Quit,
    QueryCommits(RepositoryId),
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
    LoadRepository {
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
        content_change: ReviewContent,
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
    let first_commit = git_utils::first_commit(&path).map_err(|e| ReviewHelperError::GitCommandFailed(e.to_string()))?;

    let repository_name = RepositoryName::from(name);

    let repository_store = RepositoryStore {
        base_branch: "main".to_string(),
        path,
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
                eprintln!("{}", e);
                ReviewHelperSettings::default()
            }
        };

        let ui_updater = UiUpdater::new(ui_weak);

        let storage = create_storage(app_data_path);

        let repositories = Repositories::new(storage.load_repositories().expect("Could not load repositories!"));

        {
            let ui_repositories: Vec<_> = repositories
                .iter()
                .map(|(id, repo)| (id.as_i32(), UiBasicRepository::new(repo.store())))
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
        let repository = self
            .repositories
            .get(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review = repository
            .reviews
            .get(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} in {}", review_id, repository_id));

        let file_diff = review
            .file_diffs
            .get(&file_diff_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} of {} in {}", file_diff_id, review_id, repository_id));

        let start_commit = review.diff_range().start.as_str();
        let end_commit = review.diff_range().end.as_str();
        let file = &file_diff.file_path.to_string_lossy();
        let file = file.as_ref();
        let diff_tool = self.settings.diff_tool.as_str();

        if let Err(e) = git_utils::diff_file(repository.path(), start_commit, end_commit, file, diff_tool) {
            self.ui_updater.report_error(ui::SlintResult::ShowFileDifferencesFailed, &e.to_string());
        }
    }
    fn load_commits(&self, repository_id: &RepositoryId) {
        let repository = self
            .repositories
            .get(repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        match git_utils::query_commits(repository.path()) {
            Ok(commits) => self.ui_updater.set_commits(commits),
            Err(e) => {
                self.ui_updater.clear_commits();
                self.ui_updater.report_error(ui::SlintResult::QueryingCommitsFailed, &e.to_string())
            }
        }
    }
    fn worker_loop(&mut self, mut rx: UnboundedReceiver<WorkerMessage>) {
        while let Some(message) = rx.blocking_recv() {
            match message {
                WorkerMessage::Quit => return,
                WorkerMessage::QueryCommits(repository_id) => self.load_commits(&repository_id),
                WorkerMessage::QueryDiffTools => self.query_diff_tools(),
                WorkerMessage::SaveReviewHelperSettings {
                    diff_tool,
                    editor,
                    editor_args,
                    color_scheme,
                } => self.save_settings(diff_tool, editor, editor_args, color_scheme),
                WorkerMessage::NewRepository(path) => self.new_repository(path),
                WorkerMessage::ChangeRepository { id, base_branch } => self.change_repository(id, base_branch),
                WorkerMessage::LoadRepository { id } => {
                    self.load_commits(&id);
                    self.initialize_reviews(id);
                }
                WorkerMessage::LoadReview { repository_id, review_id } => self.load_review(repository_id, review_id),
                WorkerMessage::NewReview { repository_id, name } => self.new_review(repository_id, name),
                WorkerMessage::DeleteReview { repository_id, review_id } => self.delete_review(repository_id, review_id),
                WorkerMessage::ChangeReview {
                    repository_id,
                    review_id,
                    content_change,
                } => match content_change {
                    ReviewContent::FileDiff { file_diff_id, is_reviewed } => self.change_review_file_diff(repository_id, review_id, file_diff_id, is_reviewed),
                    ReviewContent::Note { note_id, change_type } => self.change_review_notes(repository_id, review_id, note_id, change_type),
                    ReviewContent::Name(new_review_name) => self.rename_review(repository_id, review_id, new_review_name),
                },
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
            self.ui_updater.report_error(ui::SlintResult::RepositoryExists, path.to_string_lossy().as_ref());
            return;
        }
        match create_repository_store(path) {
            Ok(store) => match self.storage.save_repository(&store) {
                Ok(()) => {
                    let ui_repository = UiBasicRepository::new(&store);
                    let repository_id = self.repositories.add_repository(store);

                    self.ui_updater.new_repository(repository_id.as_i32(), ui_repository);
                }
                Err(e) => {
                    self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
                }
            },
            Err(e) => {
                self.report_review_helper_error(&e);
            }
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
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        match git_utils::repo_contains_branch(repository.path(), &base_branch) {
            Ok(contains_branch) => {
                if !contains_branch {
                    self.ui_updater.report_error(ui::SlintResult::GitBranchDoesNotExists, "Branch does not exists!");
                    return;
                }
            }
            Err(e) => {
                self.ui_updater.report_error(ui::SlintResult::GitBranchFailed, &e.to_string());
                return;
            }
        }

        let ui_base_branch = SharedString::from(&base_branch);
        repository.set_base_branch(base_branch);
        if let Err(e) = self.storage.save_repository(repository.store()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }
        self.ui_updater.change_repository(repository_id.as_usize(), ui_base_branch);
    }
    fn initialize_reviews(&mut self, repository_id: RepositoryId) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        match self.storage.load_review_names(&repository.name) {
            Ok(review_names) => {
                let mut reviews = Vec::new();
                review_names.into_iter().for_each(|review_name| {
                    let id = repository.reviews.register_review_name(review_name.clone());
                    reviews.push((id.as_i32(), SharedString::from(review_name.as_str())));
                });
                self.ui_updater.initialize_reviews(repository_id.as_usize(), reviews);
            }
            Err(e) => self.ui_updater.report_error(ui::SlintResult::LoadReviewNamesFailed, &e.to_string()),
        }
    }
    fn load_review(&mut self, repository_id: RepositoryId, review_id: ReviewId) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review_name = repository
            .reviews
            .review_name(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        let load_result = self.storage.load_review(&repository.name, review_name);
        if let Err(e) = load_result {
            self.ui_updater.report_error(ui::SlintResult::LoadReviewFailed, &e.to_string());
            return;
        }

        let Ok(Some(store)) = load_result else {
            return;
        };

        let start_diff = SharedString::from(&store.diff_range.start);
        let end_diff = SharedString::from(&store.diff_range.end);

        let review = Review::new(store, review_name.clone());

        let mut ui_notes = Vec::new();
        review.notes.iter().for_each(|id_store_tuple| {
            let context_type = if review.file_diffs.file_id_map.contains_key(&id_store_tuple.1.context) {
                SlintContextType::File
            } else {
                SlintContextType::Text
            };

            ui_notes.push(SlintNote {
                id: id_store_tuple.0.as_i32(),
                context: SharedString::from(&id_store_tuple.1.context),
                context_type,
                is_fixed: id_store_tuple.1.is_done,
                text: SharedString::from(&id_store_tuple.1.text),
            });
        });
        let ui_file_diffs: Vec<_> = review
            .file_diffs
            .iter()
            .map(|id_store_tuple| (id_store_tuple.0.as_i32(), id_store_tuple.1.clone()))
            .collect();

        repository.reviews.insert_review(review_id.clone(), review);

        self.ui_updater
            .set_review(repository_id.as_usize(), review_id.as_usize(), start_diff, end_diff, ui_notes, ui_file_diffs);
    }
    fn new_review(&mut self, repository_id: RepositoryId, name: String) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review_name = ReviewName::from(name.as_str());
        if repository.reviews.has_review_name(&review_name) {
            self.ui_updater.report_error(ui::SlintResult::ReviewAlreadyExists, &name);
            return;
        }
        if let Err(e) = self
            .storage
            .save_review_file_diffs(&repository.name, &review_name, &DiffRangeStore::default(), &[])
        {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }

        let review_id = repository.reviews.new_review(review_name);
        self.ui_updater
            .new_review(repository_id.as_usize(), review_id.as_usize(), SharedString::from(name.as_str()));
    }
    fn delete_review(&mut self, repository_id: RepositoryId, review_id: ReviewId) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review_name = repository
            .reviews
            .delete_review(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not delete {}", review_id));

        if let Err(e) = self.storage.delete_review(&repository.name, &review_name) {
            self.ui_updater.report_error(ui::SlintResult::DeleteReviewFailed, &e.to_string());
            return;
        }

        self.ui_updater.delete_review(repository_id.as_usize(), review_id.as_usize());
    }
    fn rename_review(&mut self, repository_id: RepositoryId, review_id: ReviewId, new_review_name: ReviewName) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let Some(old_review_name) = repository.reviews.rename_review(&review_id, new_review_name.clone()) else {
            return;
        };
        if let Err(e) = self.storage.rename_review(&repository.name, &old_review_name, &new_review_name) {
            self.ui_updater.report_error(ui::SlintResult::RenameReviewFailed, &e.to_string());
        }
        self.ui_updater
            .rename_review(repository_id.as_usize(), review_id.as_usize(), SharedString::from(new_review_name.as_str()));
    }
    fn change_review_notes(&mut self, repository_id: RepositoryId, review_id: ReviewId, note_id: NoteId, change_type: NoteChangeType) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review = repository
            .reviews
            .get_mut(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        let update_note_references = |note_id: &NoteId, opt_old_file_diff_id: Option<&FileDiffId>, opt_new_file_diff_id: Option<&FileDiffId>| {
            if let Some(old_file_diff_id) = opt_old_file_diff_id {
                self.ui_updater
                    .remove_note_reference(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize(), old_file_diff_id.as_usize());
            }
            if let Some(new_file_diff_id) = opt_new_file_diff_id {
                self.ui_updater
                    .add_note_reference(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize(), new_file_diff_id.as_usize());
            }
        };
        let note = review
            .notes
            .get_mut(&note_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({}, {})", note_id, review_id, repository_id));

        let mut opt_context_type = None;
        match change_type.clone() {
            NoteChangeType::Text(new_text) => note.text = new_text,
            NoteChangeType::Context(new_context) => {
                opt_context_type = if review.file_diffs.file_id_map.contains_key(&new_context) {
                    Some(SlintContextType::File)
                } else {
                    Some(SlintContextType::Text)
                };
                let new_file_diff_id = review.file_diffs.file_id_map.get(&new_context);
                let old_file_diff_id = review.file_diffs.file_id_map.get(&note.context);
                update_note_references(&note_id, old_file_diff_id, new_file_diff_id);

                note.context = new_context;
            }
            NoteChangeType::IsDone(new_is_done) => note.is_done = new_is_done,
        }
        if let Err(e) = self.storage.save_review_notes(&repository.name, review.name(), &review.notes.stores()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }
        self.ui_updater.update_note(
            repository_id.as_usize(),
            review_id.as_usize(),
            note_id.as_usize(),
            change_type,
            opt_context_type,
        );
    }
    fn change_review_file_diff(&mut self, repository_id: RepositoryId, review_id: ReviewId, file_diff_id: FileDiffId, is_reviewed: bool) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review = repository
            .reviews
            .get_mut(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        review.file_diffs.set_is_reviewed(&file_diff_id, is_reviewed);
        if let Err(e) = self
            .storage
            .save_review_file_diffs(&repository.name, review.name(), review.diff_range(), &review.file_diffs.stores())
        {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }
        self.ui_updater
            .set_file_diff_is_reviewed(repository_id.as_usize(), review_id.as_usize(), file_diff_id.as_usize(), is_reviewed);
    }
    fn find_file_difference(&mut self, repository_id: RepositoryId, review_id: ReviewId, diff_range: DiffRangeStore) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let Ok(mut file_diff_map) = git_utils::diff_git_repo(repository.path(), &diff_range.start, &diff_range.end) else {
            self.ui_updater.report_error(ui::SlintResult::FindFileDifferenceFailed, "");
            return;
        };

        let new_files = file_diff_map.keys().cloned().collect::<HashSet<_>>();

        let review = repository
            .reviews
            .get_mut(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        let (deleted_file_diff_ids, added_files) = review.file_diffs.update_file_diffs(new_files);
        review.set_diff_range(diff_range);

        let ui_file_diffs = review
            .file_diffs
            .iter()
            .map(|(id, store)| {
                let file = store.file_path.to_string_lossy().to_string();
                let diff_status = file_diff_map.remove(&file).expect("Could not found Diff-Status of cached file!");
                (id.as_i32(), store.clone(), diff_status)
            })
            .collect::<Vec<_>>();

        self.ui_updater.migrate_file_diff_notes_to_text_context(
            repository_id.as_usize(),
            review_id.as_usize(),
            deleted_file_diff_ids.into_iter().map(|id| id.as_usize()),
        );
        self.ui_updater.migrate_file_diff_notes_to_file_context(
            repository_id.as_usize(),
            review_id.as_usize(),
            added_files.into_iter().map(SharedString::from),
        );

        self.ui_updater.set_file_diffs(repository_id.as_usize(), review_id.as_usize(), ui_file_diffs);

        if let Err(e) = self
            .storage
            .save_review_file_diffs(&repository.name, review.name(), review.diff_range(), &review.file_diffs.stores())
        {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
        }
    }
    fn delete_note(&mut self, repository_id: RepositoryId, review_id: ReviewId, note_id: NoteId) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review = repository
            .reviews
            .get_mut(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        let store = review
            .notes
            .delete_note(&note_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({}, {})", note_id, review_id, repository_id));

        if let Err(e) = self.storage.save_review_notes(&repository.name, review.name(), &review.notes.stores()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }

        if let Some(file_diff_id) = review.file_diffs.file_id_map.get(&store.context) {
            self.ui_updater
                .remove_note_reference(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize(), file_diff_id.as_usize());
        }

        self.ui_updater.delete_note(repository_id.as_usize(), review_id.as_usize(), note_id.as_usize());
    }
    fn add_note(&mut self, repository_id: RepositoryId, review_id: ReviewId, text: String, context: String) {
        let repository = self
            .repositories
            .get_mut(&repository_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {}", repository_id));

        let review = repository
            .reviews
            .get_mut(&review_id)
            .unwrap_or_else(|| panic!("[BUG] Could not find {} ({})", review_id, repository_id));

        let ui_text = SharedString::from(text.as_str());
        let ui_context = SharedString::from(context.as_str());

        let opt_file_diff_id = review.file_diffs.file_id_map.get(&context).map(|id| id.as_usize());
        let note_id = review.notes.add_note(text, context);

        if let Err(e) = self.storage.save_review_notes(&repository.name, review.name(), &review.notes.stores()) {
            self.ui_updater.report_error(ui::SlintResult::StoreFailed, &e.to_string());
            return;
        }
        let context_type = if review.file_diffs.file_id_map.contains_key(ui_context.as_str()) {
            SlintContextType::File
        } else {
            SlintContextType::Text
        };

        let ui_note = SlintNote {
            id: note_id.as_i32(),
            text: ui_text,
            context: ui_context,
            context_type,
            is_fixed: false,
        };

        self.ui_updater
            .add_note(repository_id.as_usize(), review_id.as_usize(), ui_note, opt_file_diff_id);
    }
}
