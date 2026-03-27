use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{model::IdModel, ui};

pub fn is_model_set<MyModel: 'static + Model>(model: &ModelRc<MyModel::Data>) -> bool {
    model.as_any().downcast_ref::<MyModel>().is_some()
}

pub fn get_commit_model(app_window: &ui::AppWindow) -> ModelRc<ui::SlintCommit> {
    let commit_model = app_window.global::<ui::SlintCommitPickerAdapter>().get_commit_source_model();
    commit_model
}

pub fn get_review_model(app_window: &ui::AppWindow, repository_id: usize) -> ModelRc<ui::SlintReview> {
    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let Some(repository_model) = repository_model.as_any().downcast_ref::<IdModel<ui::SlintRepository>>() else {
        return ModelRc::<ui::SlintReview>::default();
    };

    match repository_model.get(repository_id) {
        Some(repository) => repository.review_model,
        None => ModelRc::<ui::SlintReview>::default(),
    }
}
pub fn get_slint_review(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> ui::SlintReview {
    let review_model = get_review_model(app_window, repository_id);
    let review_model = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>().expect("Could find model!");

    review_model
        .get(review_id)
        .unwrap_or_else(|| panic!("Could not find repository-id({})-review-id({})", repository_id, review_id))
}
pub fn get_note_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> ModelRc<ui::SlintNote> {
    let review_model = get_review_model(app_window, repository_id);
    let Some(review_model) = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>() else {
        return ModelRc::<ui::SlintNote>::default();
    };

    match review_model.get(review_id) {
        Some(review) => review.note_model,
        None => ModelRc::<ui::SlintNote>::default(),
    }
}
pub fn get_file_diff_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> ModelRc<ui::SlintFileDiff> {
    let review_model = get_review_model(app_window, repository_id);
    let Some(review_model) = review_model.as_any().downcast_ref::<IdModel<ui::SlintReview>>() else {
        return ModelRc::<ui::SlintFileDiff>::default();
    };

    match review_model.get(review_id) {
        Some(review) => review.file_diff_model,
        None => ModelRc::<ui::SlintFileDiff>::default(),
    }
}
pub fn report_error(app_window: &ui::AppWindow, error: ui::SlintResult, detail_text: SharedString) {
    let model_rc = app_window.global::<ui::SlintErrors>().get_model();
    let model = model_rc.as_any().downcast_ref::<VecModel<ui::SlintErrorEntry>>().unwrap();
    model.push(ui::SlintErrorEntry {
        error_type: error,
        text: detail_text,
    });
    app_window.invoke_request_show_error();
}
