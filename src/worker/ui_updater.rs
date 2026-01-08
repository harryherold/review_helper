use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelExt, ModelRc, SharedString, VecModel};

use crate::git_utils;
use crate::model::IdModel;
use crate::repositories::{FileDiffId, NoteId};
use crate::storage::RepositoryStore;
use crate::storage::repository_storage::{FileDiffStore, NoteStore};
use crate::ui::{self, SlintFileDiff, SlintNote, SlintReview};
use crate::worker::{NoteChangeType, ReviewHelperSettings};

pub struct UiBasicRepository {
    path: SharedString,
    name: SharedString,
    first_commit: SharedString,
    base_branch: SharedString,
}

impl UiBasicRepository {
    pub fn new(repository_store: &RepositoryStore) -> Self {
        UiBasicRepository {
            first_commit: SharedString::from(repository_store.first_commit.as_str()),
            name: SharedString::from(repository_store.name.as_str()),
            path: SharedString::from(repository_store.path.to_string_lossy().as_ref()),
            base_branch: SharedString::from(repository_store.base_branch.as_str()),
        }
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
            file_path: SharedString::from(file_diff_store.file_path.to_string_lossy().as_ref()),
            ..Default::default()
        }
    }
}

pub fn make_slint_file_diff(id: &FileDiffId, file: &String, diff_status: &git_utils::DiffStatus, is_reviewed: bool) -> SlintFileDiff {
    SlintFileDiff {
        id: id.as_i32(),
        file_path: SharedString::from(file),
        added_lines: diff_status.added_lines as i32,
        removed_lines: diff_status.removed_lines as i32,
        change_type: change_type_to_ui(&diff_status.change_type),
        is_reviewed,
    }
}

pub struct UiUpdater {
    ui_weak: slint::Weak<ui::AppWindow>,
}

impl UiUpdater {
    pub fn new(ui_weak: slint::Weak<ui::AppWindow>) -> Self {
        Self { ui_weak }
    }

    pub fn report_error(&self, error: ui::SlintResult, detail_text: &str) {
        let detail_text = SharedString::from(detail_text);
        self.ui_weak
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

    pub fn set_diff_tools(&self, diff_tools: Vec<String>) {
        self.ui_weak
            .upgrade_in_event_loop({
                let ui_diff_tools: Vec<SharedString> = diff_tools.iter().map(|t| SharedString::from(t)).collect();
                move |app_window| {
                    let model: VecModel<_> = VecModel::from(ui_diff_tools);
                    app_window.global::<ui::SlintReviewHelperSettings>().set_diff_tool_model(Rc::new(model).into());
                }
            })
            .unwrap();
    }

    pub fn initialize_review_helper_settings(&self, review_helper_settings: &ReviewHelperSettings) {
        self.ui_weak
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
    }

    pub fn initialize_repositories(&self, repositories: Vec<(i32, UiBasicRepository)>) {
        self.ui_weak
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
    pub fn new_repository(&self, repository_id: i32, ui_repository: UiBasicRepository) {
        self.ui_weak
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

    pub fn set_review(
        &self,
        repository_id: usize,
        review_id: usize,
        start_diff: SharedString,
        end_diff: SharedString,
        ui_notes: Vec<SlintNote>,
        ui_file_diffs: Vec<SlintFileDiff>,
    ) {
        self.ui_weak
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
    pub fn set_review_names(&self, repository_id: usize, reviews: Vec<(i32, SharedString)>) {
        self.ui_weak
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
    pub fn new_review(&self, repository_id: usize, review_id: usize, review_name: SharedString) {
        self.ui_weak
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

    pub fn delete_note(&self, repository_id: usize, review_id: usize, note_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let note_model = get_note_model(&app_window, repository_id, review_id);
                let note_model = note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();

                note_model.remove(note_id);
            })
            .unwrap();
    }
    pub fn update_note(&self, repository_id: usize, review_id: usize, note_id: usize, note_change_type: NoteChangeType) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let note_model = get_note_model(&app_window, repository_id, review_id);
                let note_model = note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();

                if let Some(mut note) = note_model.get(note_id) {
                    match note_change_type {
                        NoteChangeType::TextChanged(new_text) => note.text = SharedString::from(new_text),
                        NoteChangeType::ContextChanged(new_context) => note.context = SharedString::from(new_context),
                        NoteChangeType::IsDoneChanged(new_is_done) => note.is_fixed = new_is_done,
                    }
                    note_model.update(note_id, note);
                }
            })
            .unwrap();
    }

    pub fn set_file_diffs(&self, repository_id: usize, review_id: usize, ui_file_diffs: Vec<SlintFileDiff>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                let repository = repository_model.get(repository_id).expect("Repository model is out of sync with cache!");
                let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();

                assert!(review_model.has(review_id));

                let review = review_model.get(review_id).unwrap();
                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                file_diff_model.clear();

                ui_file_diffs
                    .into_iter()
                    .for_each(|ui_file_diff| file_diff_model.add(ui_file_diff.id as usize, ui_file_diff));
            })
            .unwrap();
    }
    pub fn set_file_diff_is_reviewed(&self, repository_id: usize, review_id: usize, file_diff_id: usize, is_reviewed: bool) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let file_diff_model = get_file_diff_model(&app_window, repository_id, review_id);
                let file_diff_model = file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();

                if let Some(mut file_diff) = file_diff_model.get(file_diff_id) {
                    file_diff.is_reviewed = is_reviewed;
                    file_diff_model.update(file_diff_id, file_diff);
                }
            })
            .unwrap();
    }
}

fn get_note_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> ModelRc<ui::SlintNote> {
    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();

    assert!(repository_model.has(repository_id));

    let repository = repository_model.get(repository_id).unwrap();
    let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();

    assert!(review_model.has(review_id));

    let review = review_model.get(review_id).unwrap();

    review.note_model
}
fn get_file_diff_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> ModelRc<ui::SlintFileDiff> {
    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();

    assert!(repository_model.has(repository_id));

    let repository = repository_model.get(repository_id).unwrap();
    let review_model = repository.review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();

    assert!(review_model.has(review_id));

    let review = review_model.get(review_id).unwrap();

    review.file_diff_model
}
fn change_type_to_ui(change_type: &git_utils::ChangeType) -> ui::SlintChangeType {
    match change_type {
        git_utils::ChangeType::Added => ui::SlintChangeType::Added,
        git_utils::ChangeType::Broken => ui::SlintChangeType::Broken,
        git_utils::ChangeType::Copied => ui::SlintChangeType::Copied,
        git_utils::ChangeType::Deleted => ui::SlintChangeType::Deleted,
        git_utils::ChangeType::Modified => ui::SlintChangeType::Modified,
        git_utils::ChangeType::Renamed => ui::SlintChangeType::Renamed,
        git_utils::ChangeType::TypChanged => ui::SlintChangeType::TypChanged,
        git_utils::ChangeType::Unmerged => ui::SlintChangeType::Unmerged,
        git_utils::ChangeType::Unknown => ui::SlintChangeType::Unknown,
        git_utils::ChangeType::Invalid => ui::SlintChangeType::Invalid,
    }
}
