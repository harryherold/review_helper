use std::path::PathBuf;
use std::rc::Rc;

use crate::id_model::IdModel;
use crate::ui;

#[derive(Default)]
pub struct Repositories {
    pub path: PathBuf,
    pub model: Rc<IdModel<ui::RepositoryUi>>,
}

impl Repositories {
    pub fn load_from(path: PathBuf) -> anyhow::Result<Repositories> {
        todo!()
    }
}
