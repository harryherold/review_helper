use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::model::CommitProxyModel;
use crate::ui::{self, SlintCommit};

pub struct ProxyModels {
    pub commit_proxy_model: Rc<CommitProxyModel>,
}

impl ProxyModels {
    pub fn new(app_window: &ui::AppWindow) -> Self {
        let commit_model: ModelRc<SlintCommit> = Rc::new(VecModel::default()).into();
        app_window
            .global::<ui::SlintCommitPickerAdapter>()
            .set_commit_source_model(commit_model.clone());
        Self {
            commit_proxy_model: Rc::new(CommitProxyModel::new(commit_model)),
        }
    }
}
