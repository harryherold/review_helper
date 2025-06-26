use crate::id_model::IdModel;
use crate::ui;
use crate::ui::{NoteItem, SortOrder};
use slint::{FilterModel, ModelExt, ModelRc, SharedString, SortModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

type NotesFilterModel = Rc<FilterModel<ModelRc<NoteItem>, Box<dyn Fn(&NoteItem) -> bool>>>;
type NotesSortModel = Rc<SortModel<NotesFilterModel, Box<dyn Fn(&ui::NoteItem, &ui::NoteItem) -> Ordering>>>;

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
    notes_sort_model: NotesSortModel,
}

impl NotesProxyModels {
    pub fn new(model: ModelRc<NoteItem>) -> Self {
        let note_filter = Rc::new(RefCell::new(NoteFilter::new()));
        let cloned_note_filter = note_filter.clone();
        let filter_model: NotesFilterModel = Rc::new(FilterModel::new(
            model,
            Box::new(move |item: &NoteItem| {
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
        ));

        Self {
            filtered_file_proxy_models: Rc::new(RefCell::new(HashMap::new())),
            notes_filter_model: filter_model.clone(),
            note_filter,
            notes_sort_model: Rc::new(filter_model.sort_by(Self::gen_sort_callback(ui::NoteSortCriteria::NoteText, ui::SortOrder::Ascending))),
        }
    }
    pub fn model(&self) -> ModelRc<NoteItem> {
        self.notes_sort_model.clone().into()
    }
    pub fn set_text_filter(&self, text: SharedString) {
        self.note_filter.borrow_mut().text_pattern = text;
        self.notes_filter_model.reset();
    }
    pub fn set_context_filter(&self, text: SharedString) {
        self.note_filter.borrow_mut().context_pattern = text;
        self.notes_filter_model.reset();
    }

    pub fn set_sorting(&mut self, criteria: ui::NoteSortCriteria, order: ui::SortOrder) {
        let filter_model = self.notes_filter_model.clone();
        self.notes_sort_model = Rc::new(filter_model.sort_by(Self::gen_sort_callback(criteria, order)));
    }
    fn gen_sort_callback(criteria: ui::NoteSortCriteria, order: ui::SortOrder) -> Box<dyn Fn(&ui::NoteItem, &ui::NoteItem) -> Ordering> {
        Box::new(move |lhs: &ui::NoteItem, rhs: &ui::NoteItem| -> Ordering {
            match criteria {
                ui::NoteSortCriteria::NoteText => match order { 
                    ui::SortOrder::Ascending => lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase()),
                    ui::SortOrder::Descending => rhs.text.to_lowercase().cmp(&lhs.text.to_lowercase()),
                }
                ui::NoteSortCriteria::Context => match order {
                    SortOrder::Ascending => lhs.context.to_lowercase().cmp(&rhs.context.to_lowercase()),
                    SortOrder::Descending => rhs.context.to_lowercase().cmp(&lhs.context.to_lowercase())
                }
            }
        })
    }
}

impl Default for NotesProxyModels {
    fn default() -> Self {
        let model = Rc::new(IdModel::<NoteItem>::default()).into();
        Self::new(model)
    }
}
