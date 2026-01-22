use std::collections::BTreeMap;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::model::{CommitProxyModel, FilesProxyModel};
use crate::repositories::{RepositoryId, ReviewId};
use crate::ui::{self, SlintCommit};

pub struct ReviewProxyModels {
    files_proxy_model: Rc<FilesProxyModel>,
}

impl ReviewProxyModels {
    fn new(files_source_model: ModelRc<ui::SlintFileDiff>) -> Self {
        Self {
            files_proxy_model: Rc::new(FilesProxyModel::new(files_source_model)),
        }
    }
    pub fn files_proxy_model(&self) -> Rc<FilesProxyModel> {
        self.files_proxy_model.clone()
    }
}

pub struct RepositoryProxyModels {
    id_review_models_map: BTreeMap<ReviewId, ReviewProxyModels>,
}

impl RepositoryProxyModels {
    fn new() -> Self {
        Self {
            id_review_models_map: BTreeMap::new(),
        }
    }
    pub fn review_proxy_models(&self, review_id: &ReviewId) -> Option<&ReviewProxyModels> {
        self.id_review_models_map.get(review_id)
    }
    pub fn add_review_proxy_models(&mut self, review_id: ReviewId, files_source_model: ModelRc<ui::SlintFileDiff>) {
        self.id_review_models_map.insert(review_id, ReviewProxyModels::new(files_source_model));
    }
    pub fn has_review_proxy_models(&self, review_id: &ReviewId) -> bool {
        self.id_review_models_map.contains_key(review_id)
    }
}

pub struct ProxyModels {
    id_repository_models_map: BTreeMap<RepositoryId, RepositoryProxyModels>,
    // TODO It is may be better to extract commit_proxy_model
    pub commit_proxy_model: Rc<CommitProxyModel>,
}

impl ProxyModels {
    pub fn new(app_window: &ui::AppWindow) -> Self {
        let commit_model: ModelRc<SlintCommit> = Rc::new(VecModel::default()).into();
        app_window
            .global::<ui::SlintCommitPickerAdapter>()
            .set_commit_source_model(commit_model.clone());
        Self {
            id_repository_models_map: BTreeMap::new(),
            commit_proxy_model: Rc::new(CommitProxyModel::new(commit_model)),
        }
    }
    pub fn mut_repository_proxy_models(&mut self, repository_id: &RepositoryId) -> Option<&mut RepositoryProxyModels> {
        self.id_repository_models_map.get_mut(repository_id)
    }
    pub fn repository_proxy_models(&self, repository_id: &RepositoryId) -> Option<&RepositoryProxyModels> {
        self.id_repository_models_map.get(repository_id)
    }
    pub fn add_repository_proxy_models(&mut self, repository_id: RepositoryId) {
        self.id_repository_models_map.insert(repository_id, RepositoryProxyModels::new());
    }
    pub fn has_repository_proxy_models(&self, repository_id: &RepositoryId) -> bool {
        self.id_repository_models_map.contains_key(repository_id)
    }
}
