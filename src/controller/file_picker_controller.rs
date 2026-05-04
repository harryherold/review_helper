use std::{cell::RefCell, rc::Rc};

use crate::{
    model::{IdModel, RepositoriesProxyModels, model_utils},
    repositories::{RepositoryId, ReviewId},
    ui,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString};

pub fn setup_file_picker(app_window: &ui::AppWindow, proxy_models: Rc<RefCell<RepositoriesProxyModels>>) {
    app_window.global::<ui::SlintFilePickerAdapter>().on_file_model({
        let proxy_models = proxy_models.clone();
        move |ids| -> ModelRc<SharedString> {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);

            let proxy_models = proxy_models.borrow();

            let Some(repository_proxy_models) = proxy_models.repository_proxy_models(&repository_id) else {
                return ModelRc::default();
            };

            let Some(review_proxy_model) = repository_proxy_models.review_proxy_models(&review_id) else {
                return ModelRc::default();
            };

            review_proxy_model.files_proxy_model().ui_model()
        }
    });
    app_window.global::<ui::SlintFilePickerAdapter>().on_set_filter({
        let proxy_models = proxy_models.clone();
        move |ids, pattern| {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);

            let proxy_models = proxy_models.borrow();
            let Some(repository_proxy_models) = proxy_models.repository_proxy_models(&repository_id) else {
                return;
            };
            let Some(review_proxy_models) = repository_proxy_models.review_proxy_models(&review_id) else {
                return;
            };
            let files_proxy_model = review_proxy_models.files_proxy_model();
            files_proxy_model.set_filter_pattern(pattern);
        }
    });
    app_window.global::<ui::SlintFilePickerAdapter>().on_contains_model_context({
        let app_window_weak = app_window.as_weak();
        move |ids, context| -> bool {
            let Some(app_window) = app_window_weak.upgrade() else {
                return false;
            };
            let Some(file_diff_model) = model_utils::get_file_diff_model(&app_window, ids.repository_id as usize, ids.review_id as usize) else {
                return false;
            };

            if let Some(file_diff_model) = file_diff_model.as_any().downcast_ref::<IdModel<ui::SlintFileDiff>>() {
                file_diff_model.iter().any(|file_diff| file_diff.file_path == context)
            } else {
                // TODO Add internal error!
                false
            }
        }
    });
}
