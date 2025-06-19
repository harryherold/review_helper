use crate::id_model::IdModel;
use crate::ui::NoteItem;
use slint::{FilterModel, ModelRc, SharedString};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

type NotesFilterModel = Rc<FilterModel<ModelRc<NoteItem>, Box<dyn Fn(&NoteItem) -> bool>>>;

struct NoteFilter {
    text_pattern: SharedString,
    context_pattern: SharedString,
}

impl NoteFilter {
    fn new() -> Self {
        Self {
            text_pattern: SharedString::new(),
            context_pattern: SharedString::new(),
        }
    }
}

pub struct NotesProxyModels {
    pub filtered_file_proxy_models: Rc<RefCell<HashMap<String, ModelRc<NoteItem>>>>,
    notes_filter_model: NotesFilterModel,
    note_filter: Rc<RefCell<NoteFilter>>,
}

impl NotesProxyModels {
    pub fn new(model: ModelRc<NoteItem>) -> Self {
        let note_filter = Rc::new(RefCell::new(NoteFilter::new()));
        let cloned_note_filter = note_filter.clone();
        Self {
            filtered_file_proxy_models: Rc::new(RefCell::new(HashMap::new())),
            notes_filter_model: Rc::new(FilterModel::new(
                model,
                Box::new(move |item| {
                    let note_filter = cloned_note_filter.clone();
                    let note_filter = note_filter.borrow();
                    let mut result = true;
                    if !note_filter.text_pattern.is_empty() {
                        result = result && item.text.to_lowercase().contains(&note_filter.text_pattern.to_lowercase());
                    }
                    if !note_filter.context_pattern.is_empty() {
                        result = result && item.context.to_lowercase().contains(&note_filter.context_pattern.to_lowercase());
                    }
                    result
                }),
            )),
            note_filter,
        }
    }
    pub fn model(&self) -> ModelRc<NoteItem> {
        self.notes_filter_model.clone().into()
    }
    pub fn set_text_filter(&self, text: SharedString) {
        self.note_filter.borrow_mut().text_pattern = text;
        self.notes_filter_model.reset();
    }
    pub fn set_context_filter(&self, text: SharedString) {
        self.note_filter.borrow_mut().context_pattern = text;
        self.notes_filter_model.reset();
    }
}

impl Default for NotesProxyModels {
    fn default() -> Self {
        let model = Rc::new(IdModel::<NoteItem>::default()).into();
        Self::new(model)
    }
}
