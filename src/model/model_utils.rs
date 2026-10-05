use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    model::{IdModel, SliceProxyModel},
    ui,
};

type CodeSliceProxyModel = SliceProxyModel<Rc<VecModel<ui::SlintDiffLine>>>;

const BUFFER_COUNT: usize = 10;

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

pub fn update_diff_model(app_window: &ui::AppWindow, model: Rc<CodeSliceProxyModel>, viewport_y: f32, available_height: f32) -> f32 {
    let style = app_window.global::<ui::Style>();

    let line_height = style.get_size().file_diff_line_height;

    let first_visible_idx = (viewport_y / line_height).floor() as usize;
    let visible_count = (available_height / line_height).ceil() as usize;

    let source_model = model.source();

    let start_idx = first_visible_idx.saturating_sub(BUFFER_COUNT);
    let end_idx = (first_visible_idx + visible_count + BUFFER_COUNT).min(source_model.row_count());

    let count = end_idx.saturating_sub(start_idx);

    model.set_slice(start_idx, count);

    // app.set_visible_start_y(start_y_px);

    // let edited_row = app.get_edited_row();
    // if edited_row != -1 {
    //     let edited_row = edited_row as usize;
    //     if edited_row < start_idx || edited_row >= end_idx {
    //         app.set_edited_row(-1);
    //     }
    // }
    start_idx as f32 * line_height
}
