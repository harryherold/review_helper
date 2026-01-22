use std::{cell::RefCell, rc::Rc};

use crate::{
    model::{IdModel, ProxyModels, model_utils},
    repositories::{RepositoryId, ReviewId},
    ui,
};
use slint::{ComponentHandle, ModelRc, SharedString};

pub fn setup_file_picker(app_window: &ui::AppWindow, proxy_models: Rc<RefCell<ProxyModels>>) {
    app_window.global::<ui::SlintFilePickerAdapter>().on_file_model({
        let ui_weak = app_window.as_weak();
        let proxy_models = proxy_models.clone();
        move |ids| -> ModelRc<SharedString> {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);

            let mut proxy_models = proxy_models.borrow_mut();

            if !proxy_models.has_repository_proxy_models(&repository_id) {
                proxy_models.add_repository_proxy_models(repository_id.clone());
            }

            let repository_proxy_models = proxy_models.mut_repository_proxy_models(&repository_id).expect("Add repository failed!");

            if !repository_proxy_models.has_review_proxy_models(&review_id) {
                let ui = ui_weak.unwrap();
                let file_diff_model = model_utils::get_file_diff_model(&ui, repository_id.as_usize(), review_id.as_usize());
                if !model_utils::is_model_set::<IdModel<ui::SlintFileDiff>>(&file_diff_model) {
                    model_utils::report_error(&ui, ui::SlintResult::InvalidModel, SharedString::from("IdModel<ui::SlintFileDiff>"));
                    return ModelRc::<SharedString>::default();
                }
                repository_proxy_models.add_review_proxy_models(review_id.clone(), file_diff_model);
            }
            let files_proxy_model = repository_proxy_models
                .review_proxy_models(&review_id)
                .expect("Add review failed!")
                .files_proxy_model();

            files_proxy_model.ui_model()
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
}
