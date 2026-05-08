use crate::ui;
use crate::ui::{SlintNote, SlintSortOrder};
use slint::{FilterModel, ModelRc, SharedString, SortModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type NotesFilterModel = Rc<FilterModel<ModelRc<SlintNote>, Box<dyn Fn(&SlintNote) -> bool>>>;
type NotesSortModel = Rc<SortModel<NotesFilterModel, Box<dyn Fn(&ui::SlintNote, &ui::SlintNote) -> Ordering>>>;

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

struct NoteSortParameter {
    order: ui::SlintSortOrder,
    criteria: ui::SlintNoteSortCriteria,
}

impl NoteSortParameter {
    fn new() -> Self {
        Self {
            order: ui::SlintSortOrder::Ascending,
            criteria: ui::SlintNoteSortCriteria::Text,
        }
    }
}

pub struct NotesProxyModels {
    notes_filter_model: NotesFilterModel,
    note_filter: Rc<RefCell<NoteFilter>>,
    notes_sort_model: NotesSortModel,
    note_sort_parameter: Rc<RefCell<NoteSortParameter>>,
}

impl NotesProxyModels {
    pub fn new(source_model: ModelRc<SlintNote>) -> Self {
        let filter = Rc::new(RefCell::new(NoteFilter::new()));

        let filter_callback = Box::new({
            let note_filter = filter.clone();
            move |note: &SlintNote| {
                let note_filter = note_filter.borrow();
                let mut result = true;
                if !note_filter.text_pattern.is_empty() {
                    result = result && note.text.to_lowercase().contains(&note_filter.text_pattern.to_lowercase());
                }
                if !note_filter.context_pattern.is_empty() {
                    result = result && note.context.to_lowercase().contains(&note_filter.context_pattern.to_lowercase());
                }
                result
            }
        });

        let filter_model: NotesFilterModel = Rc::new(FilterModel::new(source_model, filter_callback));

        let sort_parameter = Rc::new(RefCell::new(NoteSortParameter::new()));

        let sort_callback = Box::new({
            let sort_parameter = sort_parameter.clone();
            move |lhs: &ui::SlintNote, rhs: &ui::SlintNote| -> Ordering {
                let sort_parameter = sort_parameter.borrow();
                match sort_parameter.criteria {
                    ui::SlintNoteSortCriteria::Text => match sort_parameter.order {
                        ui::SlintSortOrder::Ascending => lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase()),
                        ui::SlintSortOrder::Descending => rhs.text.to_lowercase().cmp(&lhs.text.to_lowercase()),
                    },
                    ui::SlintNoteSortCriteria::Context => match sort_parameter.order {
                        SlintSortOrder::Ascending => lhs.context.to_lowercase().cmp(&rhs.context.to_lowercase()),
                        SlintSortOrder::Descending => rhs.context.to_lowercase().cmp(&lhs.context.to_lowercase()),
                    },
                }
            }
        });

        let sort_model: NotesSortModel = Rc::new(SortModel::new(filter_model.clone(), sort_callback));

        Self {
            notes_filter_model: filter_model,
            note_filter: filter,
            notes_sort_model: sort_model,
            note_sort_parameter: sort_parameter,
        }
    }
    pub fn ui_model(&self) -> ModelRc<SlintNote> {
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
    pub fn set_sort_parameter(&self, criteria: ui::SlintNoteSortCriteria, order: ui::SlintSortOrder) {
        let mut sort_parameter = self.note_sort_parameter.borrow_mut();
        sort_parameter.criteria = criteria;
        sort_parameter.order = order;
        self.notes_sort_model.reset();
    }
}
