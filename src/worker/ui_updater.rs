use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

use itertools::Itertools;
use slint::ModelRc;
use slint::{ComponentHandle, Model, ModelExt, SharedString, VecModel};

use crate::git_utils;
use crate::git_utils::DiffStatus;
use crate::model::IdModel;
use crate::model::model_utils;
use crate::repositories::FileDiffId;
use crate::storage::RepositoryStore;
use crate::storage::repository_storage::FileDiffStore;
use crate::ui::SlintChangeTypeOccurrence;
use crate::ui::SlintContextType;
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
        ui_file_diffs: Vec<(i32, FileDiffStore)>,
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

                let mut file_notes_map: HashMap<String, Rc<VecModel<i32>>> = HashMap::new();

                let notes_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                ui_notes.into_iter().enumerate().for_each(|(index, ui_note)| {
                    if ui_note.context_type == SlintContextType::File {
                        file_notes_map
                            .entry(ui_note.context.to_string())
                            .and_modify(|e| e.push(index as i32))
                            .or_insert(Rc::new(VecModel::from(vec![index as i32])));
                    }
                    if ui_note.is_fixed {
                        review.note_progress.completed_count += 1;
                    }
                    notes_model.add(ui_note.id as usize, ui_note)
                });

                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                ui_file_diffs.into_iter().for_each(|(file_diff_id, store)| {
                    if store.is_reviewed {
                        review.review_progress.completed_count += 1;
                    }

                    let file_path = store.file_path.to_string_lossy().to_string();
                    let referenced_notes = file_notes_map.remove(&file_path).unwrap_or_else(|| Rc::new(VecModel::default()));

                    file_diff_model.add(
                        file_diff_id as usize,
                        SlintFileDiff {
                            id: file_diff_id,
                            file_path: SharedString::from(file_path),
                            is_reviewed: store.is_reviewed,
                            referenced_notes: referenced_notes.into(),
                            ..Default::default()
                        },
                    );
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
    pub fn rename_review(&self, repository_id: usize, review_id: usize, new_review_name: SharedString) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();
                review.name = new_review_name;
                review_model.update(review_id, review);
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
    pub fn update_note(
        &self,
        repository_id: usize,
        review_id: usize,
        note_id: usize,
        note_change_type: NoteChangeType,
        opt_context_type: Option<SlintContextType>,
    ) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();

                let note_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();

                if let Some(mut note) = note_model.get(note_id) {
                    match note_change_type {
                        NoteChangeType::TextChanged(ref new_text) => note.text = SharedString::from(new_text),
                        NoteChangeType::ContextChanged(ref new_context) => {
                            if let Some(new_context_type) = opt_context_type {
                                note.context_type = new_context_type;
                            }
                            note.context = SharedString::from(new_context);
                        }
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

    fn note_id_to_index(review: &SlintReview, note_id: usize) -> i32 {
        let note_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
        note_model.id_to_index(note_id)
    }

    fn get_note_references(review: &SlintReview, file_diff_id: usize) -> ModelRc<i32> {
        let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
        let Some(file_diff) = file_diff_model.get(file_diff_id) else {
            panic!("Could not find file diff id {}", file_diff_id);
        };
        file_diff.referenced_notes
    }

    pub fn migrate_file_diff_notes_to_file_context<I>(&self, repository_id: usize, review_id: usize, added_files: I)
    where
        I: IntoIterator<Item = SharedString> + Send + 'static,
        I::IntoIter: Send,
    {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let note_model = model_utils::get_note_model(&app_window, repository_id, review_id);
                let note_model = note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                added_files.into_iter().for_each(|file| {
                    note_model.iter().for_each(|mut note| {
                        if note.context_type == ui::SlintContextType::Text && note.context == file {
                            note.context_type = ui::SlintContextType::File;
                            note_model.update(note.id as usize, note);
                        }
                    });
                });
            })
            .unwrap();
    }

    pub fn migrate_file_diff_notes_to_text_context<I>(&self, repository_id: usize, review_id: usize, deleted_file_ids: I)
    where
        I: IntoIterator<Item = usize> + Send + 'static,
        I::IntoIter: Send,
    {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review = model_utils::get_slint_review(&app_window, repository_id, review_id);
                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                let mut note_indexes = Vec::new();
                deleted_file_ids.into_iter().for_each(|id| {
                    if let Some(file_diff) = file_diff_model.get(id) {
                        note_indexes.extend(file_diff.referenced_notes.iter());
                    }
                });
                let notes_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                note_indexes.into_iter().for_each(|note_index| {
                    if let Some(mut note) = notes_model.row_data(note_index as usize) {
                        note.context_type = ui::SlintContextType::Text;
                        notes_model.set_row_data(note_index as usize, note);
                    }
                });
            })
            .unwrap();
    }

    pub fn add_note_reference(&self, repository_id: usize, review_id: usize, note_id: usize, file_diff_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review = model_utils::get_slint_review(&app_window, repository_id, review_id);
                let note_index = Self::note_id_to_index(&review, note_id);
                if note_index < 0 {
                    return;
                }
                let referenced_notes_model = Self::get_note_references(&review, file_diff_id);
                let referenced_notes_model = referenced_notes_model.as_any().downcast_ref::<VecModel<i32>>().unwrap();
                referenced_notes_model.push(note_index);
            })
            .unwrap();
    }
    pub fn remove_note_reference(&self, repository_id: usize, review_id: usize, note_id: usize, file_diff_id: usize) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review = model_utils::get_slint_review(&app_window, repository_id, review_id);
                let note_index = Self::note_id_to_index(&review, note_id);
                if note_index < 0 {
                    return;
                }
                let referenced_notes_model = Self::get_note_references(&review, file_diff_id);
                let referenced_notes_model = referenced_notes_model.as_any().downcast_ref::<VecModel<i32>>().unwrap();
                let remove_index = referenced_notes_model
                    .iter()
                    .position(|i| i == note_index)
                    .unwrap_or_else(|| panic!("Could not find referenced note index {} ({}, {})", note_index, review_id, repository_id));
                referenced_notes_model.remove(remove_index);
            })
            .unwrap();
    }

    pub fn set_file_diffs(&self, repository_id: usize, review_id: usize, ui_file_diffs: Vec<(i32, FileDiffStore, DiffStatus)>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model
                    .get(review_id)
                    .unwrap_or_else(|| panic!("Could not find repository-id({})-review-id({})", repository_id, review_id));

                let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                file_diff_model.clear();

                review.difference_statistics.added_lines = 0;
                review.difference_statistics.removed_lines = 0;

                let mut change_type_map = BTreeMap::new();

                review.review_progress.total_count = ui_file_diffs.len() as i32;
                review.review_progress.completed_count = 0;

                let mut file_notes_index_map: HashMap<String, Rc<VecModel<i32>>> = HashMap::new();
                review.note_model.iter().enumerate().for_each(|(index, note)| {
                    if note.context_type == SlintContextType::File {
                        file_notes_index_map
                            .entry(note.context.to_string())
                            .and_modify(|e| e.push(index as i32))
                            .or_insert(Rc::new(VecModel::from(vec![index as i32])));
                    }
                });

                let mut add_statistics = |store: &FileDiffStore, status: &DiffStatus| {
                    let ui_change_type = change_type_to_ui(&status.change_type);
                    review.difference_statistics.added_lines += status.added_lines as i32;
                    review.difference_statistics.removed_lines += status.removed_lines as i32;
                    change_type_map
                        .entry(ui_change_type as usize)
                        .and_modify(|e: &mut (i32, ui::SlintChangeType)| e.0 += 1)
                        .or_insert((1, ui_change_type));

                    if store.is_reviewed {
                        review.review_progress.completed_count += 1;
                    }
                };

                let mut add_to_file_diff_model = |file_diff_id: i32, store: FileDiffStore, status: DiffStatus| {
                    let file_path = store.file_path.to_string_lossy().to_string();
                    let referenced_notes = file_notes_index_map.remove(&file_path).unwrap_or_else(|| Rc::new(VecModel::default()));
                    file_diff_model.add(
                        file_diff_id as usize,
                        SlintFileDiff {
                            id: file_diff_id,
                            added_lines: status.added_lines as i32,
                            removed_lines: status.removed_lines as i32,
                            change_type: change_type_to_ui(&status.change_type),
                            file_path: SharedString::from(file_path),
                            is_reviewed: store.is_reviewed,
                            referenced_notes: referenced_notes.into(),
                        },
                    );
                };

                ui_file_diffs.into_iter().for_each(|(file_diff_id, store, status)| {
                    add_statistics(&store, &status);
                    add_to_file_diff_model(file_diff_id, store, status);
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
    pub fn add_note(&self, repository_id: usize, review_id: usize, note: SlintNote, opt_file_diff_id: Option<usize>) {
        self.ui_weak
            .upgrade_in_event_loop(move |app_window| {
                let review_model = model_utils::get_review_model(&app_window, repository_id);
                let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().unwrap();
                let mut review = review_model.get(review_id).unwrap();

                let note_model = review.note_model.as_any().downcast_ref::<IdModel<ui::SlintNote>>().unwrap();
                note_model.add(note.id.clone() as usize, note);

                review.note_progress.total_count += 1;

                if let Some(file_diff_id) = opt_file_diff_id {
                    let file_diff_model = review.file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>().unwrap();
                    if let Some(file_diff) = file_diff_model.get(file_diff_id as usize) {
                        let referenced_notes_model = file_diff.referenced_notes.as_any().downcast_ref::<VecModel<i32>>().unwrap();
                        let note_index = note_model.row_count() - 1;
                        referenced_notes_model.push(note_index as i32);
                    }
                }

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
