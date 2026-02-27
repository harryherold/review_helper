use std::collections::BTreeMap;
use std::collections::HashSet;
use std::rc::Rc;

use itertools::Itertools;
use slint::{ComponentHandle, Model, ModelExt, SharedString, VecModel};

use crate::git_utils;
use crate::model::IdModel;
use crate::model::model_utils;
use crate::repositories::{FileDiffId, NoteId};
use crate::storage::RepositoryStore;
use crate::storage::repository_storage::{FileDiffStore, NoteStore};
use crate::ui::SlintChangeTypeOccurrence;
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
                model_utils::report_error(&app_window, error, detail_text);
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
    pub fn delete_repository(&self, repository_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop({
                move |app_window| {
                    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                    let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                    repository_model.remove(repository_id);
                }
            })
            .unwrap();
    }
    pub fn change_repository(&self, repository_id: usize, base_branch: SharedString) {
        self.ui_weak
            .upgrade_in_event_loop({
                move |app_window| {
                    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
                    let repository_model = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>().unwrap();
                    if let Some(mut repository) = repository_model.get(repository_id) {
                        repository.base_branch = base_branch;
                        repository_model.update(repository_id, repository);
                    }
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
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).expect("Review model is out of sync with cache");
                review.start_diff = start_diff;
                review.end_diff = end_diff;
                review.is_loaded = true;
                review.review_progress.total_count = ui_file_diffs.len() as i32;
                review.note_progress.total_count = ui_notes.len() as i32;

                let notes_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                ui_notes.into_iter().for_each(|ui_note| {
                    if ui_note.is_fixed {
                        review.note_progress.completed_count += 1;
                    }
                    notes_model.add(ui_note.id as usize, ui_note)
                });

                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                ui_file_diffs.into_iter().for_each(|ui_file_diff| {
                    if ui_file_diff.is_reviewed {
                        review.review_progress.completed_count += 1;
                    }
                    file_diff_model.add(ui_file_diff.id as usize, ui_file_diff)
                });

                review_model.update(review_id, review);
            })
            .unwrap();
    }
    pub fn initialize_reviews(&self, repository_id: usize, reviews: Vec<(i32, SharedString)>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                reviews.into_iter().for_each(|(id, name)| {
                    review_model.add(
                        id as usize,
                        ui::SlintReview {
                            id,
                            name: name.clone(),
                            note_model: Rc::new(IdModel::default()).into(),
                            file_diff_model: Rc::new(IdModel::default()).into(),
                            is_loaded: false,
                            difference_statistics: ui::SlintDifferenceStatistics {
                                added_lines: 0,
                                removed_lines: 0,
                                change_type_model: Rc::new(VecModel::default()).into(),
                            },
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
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();

                assert!(false == review_model.has(review_id));

                review_model.add(
                    review_id,
                    SlintReview {
                        id: review_id as i32,
                        name: review_name,
                        note_model: Rc::new(IdModel::default()).into(),
                        file_diff_model: Rc::new(IdModel::default()).into(),
                        is_loaded: true,
                        difference_statistics: ui::SlintDifferenceStatistics {
                            added_lines: 0,
                            removed_lines: 0,
                            change_type_model: Rc::new(VecModel::default()).into(),
                        },
                        ..Default::default()
                    },
                );

                app_window
                    .global::<ui::SlintReviewCallbacks>()
                    .invoke_initialize_ui_models(ui::SlintReviewIdParameters {
                        repository_id: repository_id as i32,
                        review_id: review_id as i32,
                    });
            })
            .unwrap();
    }
    pub fn delete_review(&self, repository_id: usize, review_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                review_model.remove(review_id);
            })
            .unwrap();
    }

    pub fn delete_note(&self, repository_id: usize, review_id: usize, note_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let note_model = model_utils::get_note_model(&app_window, repository_id, review_id);
                let note_model = note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();

                note_model.remove(note_id);
            })
            .unwrap();
    }
    pub fn update_note(&self, repository_id: usize, review_id: usize, note_id: usize, note_change_type: NoteChangeType) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();

                let note_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();

                if let Some(mut note) = note_model.get(note_id) {
                    match note_change_type {
                        NoteChangeType::TextChanged(ref new_text) => note.text = SharedString::from(new_text),
                        NoteChangeType::ContextChanged(ref new_context) => note.context = SharedString::from(new_context),
                        NoteChangeType::IsDoneChanged(new_is_done) => note.is_fixed = new_is_done,
                    }
                    note_model.update(note_id, note);

                    if let NoteChangeType::IsDoneChanged(is_done) = note_change_type {
                        review.note_progress.completed_count = if is_done {
                            review.note_progress.completed_count + 1
                        } else {
                            review.note_progress.completed_count - 1
                        };
                        review_model.update(review_id, review);
                    }
                }
            })
            .unwrap();
    }

    pub fn set_file_diffs(&self, repository_id: usize, review_id: usize, ui_file_diffs: Vec<SlintFileDiff>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let Some(mut review) = model_utils::get_slint_review(&app_window, repository_id, review_id) else {
                    model_utils::report_error(
                        &app_window,
                        ui::SlintResult::ModelItemNotExists,
                        SharedString::from(format!("repository id {} review id {}", repository_id, review_id)),
                    );
                    return;
                };

                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                file_diff_model.clear();

                review.difference_statistics.added_lines = 0;
                review.difference_statistics.removed_lines = 0;

                let mut change_type_map: BTreeMap<usize, (i32, ui::SlintChangeType)> = BTreeMap::new();

                review.review_progress.total_count = ui_file_diffs.len() as i32;
                review.review_progress.completed_count = 0;

                ui_file_diffs.into_iter().for_each(|ui_file_diff| {
                    review.difference_statistics.added_lines += ui_file_diff.added_lines;
                    review.difference_statistics.removed_lines += ui_file_diff.removed_lines;
                    change_type_map
                        .entry(ui_file_diff.change_type as usize)
                        .and_modify(|e| e.0 += 1)
                        .or_insert((1, ui_file_diff.change_type));

                    if ui_file_diff.is_reviewed {
                        review.review_progress.completed_count += 1;
                    }
                    file_diff_model.add(ui_file_diff.id as usize, ui_file_diff);
                });

                let change_type_model = review
                    .difference_statistics
                    .change_type_model
                    .as_any()
                    .downcast_ref::<VecModel<ui::SlintChangeTypeOccurrence>>()
                    .unwrap();
                change_type_model.clear();
                change_type_map.into_values().for_each(|(count, change_type)| {
                    change_type_model.push(SlintChangeTypeOccurrence { change_type, count });
                });

                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                review_model.update(review_id, review);
            })
            .unwrap();
    }
    pub fn set_file_diff_is_reviewed(&self, repository_id: usize, review_id: usize, file_diff_id: usize, is_reviewed: bool) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();

                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();

                if let Some(mut file_diff) = file_diff_model.get(file_diff_id) {
                    file_diff.is_reviewed = is_reviewed;
                    file_diff_model.update(file_diff_id, file_diff);

                    review.review_progress.completed_count = if is_reviewed {
                        review.review_progress.completed_count + 1
                    } else {
                        review.review_progress.completed_count - 1
                    };

                    review_model.update(review_id, review);
                }
            })
            .unwrap();
    }
    pub fn add_note(&self, repository_id: usize, review_id: usize, note: SlintNote) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();

                let note_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                note_model.add(note.id.clone() as usize, note);

                review.note_progress.total_count += 1;
                review_model.update(review_id, review);
            })
            .unwrap();
    }

    pub fn set_commits(&self, commits: Vec<git_utils::Commit>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                //TODO <All> must be translated
                let mut authors = vec![SharedString::from("All")];
                let author_set: HashSet<_> = commits.iter().map(|c| SharedString::from(&c.author)).collect();
                authors.append(&mut author_set.into_iter().sorted().collect::<Vec<_>>());

                let author_model = app_window.global::<ui::SlintCommitPickerAdapter>().get_author_model();
                let author_model = author_model.as_any().downcast_ref::<VecModel<SharedString>>().unwrap();
                author_model.set_vec(authors);

                let ui_commits = commits.into_iter().map(|commit| ui::SlintCommit::from(commit)).collect::<Vec<_>>();

                let commit_model = model_utils::get_commit_model(&app_window);
                let commit_model = commit_model.as_any().downcast_ref::<VecModel<ui::SlintCommit>>().unwrap();

                commit_model.clear();
                commit_model.set_vec(ui_commits);
            })
            .unwrap();
    }
    pub fn clear_commits(&self) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let commit_model = model_utils::get_commit_model(&app_window);
                let commit_model = commit_model.as_any().downcast_ref::<VecModel<ui::SlintCommit>>().unwrap();
                commit_model.clear();
            })
            .unwrap();
    }
}

impl From<git_utils::Commit> for ui::SlintCommit {
    fn from(value: git_utils::Commit) -> Self {
        ui::SlintCommit {
            commit_id: SharedString::from(value.hash.as_str()),
            author: SharedString::from(value.author.as_str()),
            date: SharedString::from(value.date.as_str()),
            message: SharedString::from(value.message.as_str()),
        }
    }
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
