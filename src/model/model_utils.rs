use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{model::IdModel, ui};

#[macro_export]
macro_rules! cast_model {
    ($any_model:expr, $to_type:ty) => {
        $any_model.as_any().downcast_ref::<$to_type>().expect("[BUG] downcast_ref failed!")
    };
}

pub fn get_review_model(app_window: &ui::AppWindow, repository_id: usize) -> Option<ModelRc<ui::SlintReview>> {
    let repository_model = app_window.global::<ui::SlintReviewHelper>().get_repositories();
    let repository_model = cast_model!(repository_model, IdModel<ui::SlintRepository>);

    let repository = repository_model.get(repository_id)?;
    Some(repository.review_model)
}

pub fn get_slint_review(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> Option<ui::SlintReview> {
    let review_model = get_review_model(app_window, repository_id)?;
    let review_model = cast_model!(review_model, IdModel<ui::SlintReview>);

    review_model.get(review_id)
}
pub fn get_note_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> Option<ModelRc<ui::SlintNote>> {
    let review_model = get_review_model(app_window, repository_id)?;
    let review_model = cast_model!(review_model, IdModel<ui::SlintReview>);

    let review = review_model.get(review_id)?;
    Some(review.note_model)
}

pub fn get_file_diff_model(app_window: &ui::AppWindow, repository_id: usize, review_id: usize) -> Option<ModelRc<ui::SlintFileDiff>> {
    let review_model = get_review_model(app_window, repository_id)?;
    let review_model = cast_model!(review_model, IdModel<ui::SlintReview>);

    let review = review_model.get(review_id)?;
    Some(review.file_diff_model)
}

pub fn report_error(app_window: &ui::AppWindow, error: ui::SlintResult, detail_text: SharedString) {
    let model_rc = app_window.global::<ui::SlintErrors>().get_model();
    let model = cast_model!(model_rc, VecModel<ui::SlintErrorEntry>);
    model.push(ui::SlintErrorEntry {
        error_type: error,
        text: detail_text,
    });
    app_window.invoke_request_show_error();
}
