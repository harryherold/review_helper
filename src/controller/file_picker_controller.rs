use std::{cell::RefCell, rc::Rc};

use crate::{
    model::ProxyModels,
    repositories::{RepositoryId, ReviewId},
    ui,
};
use slint::{ComponentHandle, ModelRc, SharedString};

pub fn setup_file_picker(app_window: &ui::AppWindow, proxy_models: Rc<RefCell<ProxyModels>>) {
    app_window.global::<ui::SlintFilePickerAdapter>().on_file_model({
        let proxy_models = proxy_models.clone();
        move |ids| -> ModelRc<SharedString> {
            let repository_id = RepositoryId::from(ids.repository_id);
            let review_id = ReviewId::from(ids.review_id);

            let proxy_models = proxy_models.borrow();

            let repository_proxy_models = proxy_models.repository_proxy_models(&repository_id).expect("Could not find repository!");

            let files_proxy_model = repository_proxy_models
                .review_proxy_models(&review_id)
                .expect("Could not find review!")
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
