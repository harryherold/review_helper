use crate::ui::NoteItem;
use slint::ModelRc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct NotesProxyModels {
    pub filtered_file_proxy_models: Rc<RefCell<HashMap<String, ModelRc<NoteItem>>>>,
}

impl Default for NotesProxyModels {
    fn default() -> Self {
        NotesProxyModels {
            filtered_file_proxy_models: Rc::new(RefCell::new(HashMap::new())),
        }
    }
}
