use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    cast_model,
    model::{IdModel, SliceProxyModel, model_utils},
    repositories::{FileDiffId, RepositoryId, ReviewId},
    ui, unwrap_or_return,
    worker::{WorkerChannel, WorkerMessage},
};

pub fn setup_file_diff_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintFileDiffCallbacks>().on_request_update_diff_model({
        let app_window_weak = app_window.as_weak();
        move |buffered_lines, viewport_y, available_height| -> f32 {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", 0.0);

            let buffered_lines = cast_model!(buffered_lines, SliceProxyModel<Rc<VecModel<ui::SlintDiffLine>>>);
            model_utils::update_diff_model(&app_window, buffered_lines, viewport_y, available_height)
        }
    });
    app_window.global::<ui::SlintFileDiffCallbacks>().on_create_or_get_diff_model_entry({
        let app_window_weak = app_window.as_weak();
        let channel = worker_channel.clone();
        move |ids| -> i32 {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", -1);
            let review_model = model_utils::get_review_model(&app_window, ids.review_id_parameters.repository_id as usize)
                .unwrap_or_else(|| panic!("[BUG] RepositoryId {} not found", ids.review_id_parameters.repository_id));
            let review_model = cast_model!(review_model, IdModel<ui::SlintReview>);
            let review = review_model
                .get(ids.review_id_parameters.review_id as usize)
                .unwrap_or_else(|| panic!("[BUG] ReviewId {} not found", ids.review_id_parameters.review_id));

            let loaded_file_diffs = cast_model!(review.loaded_file_diffs, IdModel<ui::SlintDiffLines>);

            if !loaded_file_diffs.has(ids.file_diff_id as usize) {
                let lines = Rc::new(VecModel::default());
                let lines_proxy = Rc::new(SliceProxyModel::new(lines.clone()));

                let diff_lines = ui::SlintDiffLines {
                    max_chars_count: 0,
                    lines: lines.into(),
                    buffered_lines: lines_proxy.into(),
                    mini_map_segments: Rc::new(VecModel::default()).into(),
                };

                loaded_file_diffs.add(ids.file_diff_id as usize, diff_lines.clone());
                let message = WorkerMessage::LoadFileLineDifferences {
                    repository_id: RepositoryId::from(ids.review_id_parameters.repository_id),
                    review_id: ReviewId::from(ids.review_id_parameters.review_id),
                    file_diff_id: FileDiffId::from(ids.file_diff_id),
                };
                channel.send(message).expect("Worker channel broken!");
            }
            loaded_file_diffs.id_to_index(ids.file_diff_id as usize).map(|v| v as i32).unwrap_or(-1)
        }
    });
    app_window.global::<ui::SlintFileDiffCallbacks>().on_loaded_file_diffs_model({
        let app_window_weak = app_window.as_weak();
        move |ids| -> ModelRc<ui::SlintDiffLines> {
            let app_window = unwrap_or_return!(app_window_weak.upgrade(), "Upgrade to AppWindow failed!", ModelRc::default());
            let repository_id = ids.repository_id as usize;
            let review_id = ids.review_id as usize;
            let review_model =
                model_utils::get_review_model(&app_window, repository_id).unwrap_or_else(|| panic!("[BUG] RepositoryId {} not found", repository_id));
            let review_model = cast_model!(review_model, IdModel<ui::SlintReview>);
            let review = review_model.get(review_id).unwrap_or_else(|| panic!("[BUG] ReviewId {} not found", review_id));
            review.loaded_file_diffs
        }
    })
}
