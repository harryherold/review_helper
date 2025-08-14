use crate::id_model::IdModel;
use crate::ui;
use slint::{FilterModel, ModelExt, ModelRc, SharedString, SortModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;

type FileDiffFilterModel = Rc<FilterModel<ModelRc<ui::DiffFileItem>, Box<dyn Fn(&ui::DiffFileItem) -> bool>>>;
type FileDiffSortModel = Rc<SortModel<FileDiffFilterModel, fn(&ui::DiffFileItem, &ui::DiffFileItem) -> Ordering>>;

pub struct FileDiffProxyModels {
    filter_model: FileDiffFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    filter_review_state: Rc<RefCell<ui::FilterReviewState>>,
    sort_model: FileDiffSortModel,
}

impl FileDiffProxyModels {
    fn sort_by_name(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
        lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
    }
    fn sort_by_extension(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
        let lhs_opt = extension_from_filename(&lhs.text);
        let rhs_opt = extension_from_filename(&rhs.text);
        if lhs_opt.is_some() && rhs_opt.is_some() {
            let result = lhs_opt.unwrap().cmp(rhs_opt.unwrap());
            if result == Ordering::Equal {
                lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
            } else {
                result
            }
        } else if lhs_opt.is_some() && rhs_opt.is_none() {
            Ordering::Greater
        } else if lhs_opt.is_none() && rhs_opt.is_some() {
            Ordering::Less
        } else {
            lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
        }
    }
    fn sort_by_is_done(lhs: &ui::DiffFileItem, rhs: &ui::DiffFileItem) -> Ordering {
        let lhs_is_done = lhs.is_reviewed;
        let rhs_is_done = rhs.is_reviewed;
        
        if lhs_is_done && !rhs_is_done {
            return Ordering::Less;
        } else if !lhs_is_done && rhs_is_done {
            return Ordering::Greater;
        } else {
            lhs.text.to_lowercase().cmp(&rhs.text.to_lowercase())
        }
    }

    pub fn new(model: ModelRc<ui::DiffFileItem>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let filter_review_state = Rc::new(RefCell::new(ui::FilterReviewState::Unfiltered)); 
        let clone_filter_text = filter_text.clone();
        let clone_filter_review_state = filter_review_state.clone();

        let fm: FileDiffFilterModel = Rc::new(FilterModel::new(
            model,
            Box::new(move |item: &ui::DiffFileItem| -> bool {
                let filter_review_state = filter_review_state.clone();
                let filter_review_state = *filter_review_state.borrow();
                if filter_review_state == ui::FilterReviewState::Done && !item.is_reviewed {
                    return false;
                }
                if filter_review_state == ui::FilterReviewState::Open && item.is_reviewed {
                    return false;
                }
                let filter_text = filter_text.clone();
                let pattern = filter_text.borrow();
                if pattern.is_empty() {
                    return true;
                } else {
                    item.text.to_lowercase().contains(&pattern.as_str().to_lowercase())
                }
            }),
        ));

        FileDiffProxyModels {
            filter_model: fm.clone(),
            filter_text: clone_filter_text,
            filter_review_state: clone_filter_review_state,
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }

    pub fn sort_by(&mut self, sort_criteria: ui::SortCriteria) {
        self.sort_model = match sort_criteria {
            ui::SortCriteria::Name =>  Rc::new(self.filter_model.clone().sort_by(Self::sort_by_name)),
            ui::SortCriteria::Extension =>  Rc::new(self.filter_model.clone().sort_by(Self::sort_by_extension)),
            ui::SortCriteria::IsDone =>  Rc::new(self.filter_model.clone().sort_by(Self::sort_by_is_done)),
        };
    }

    pub fn set_filter_text(&mut self, filter_text: SharedString) {
        *self.filter_text.borrow_mut() = filter_text;
        self.filter_model.reset();
    }
    
    pub fn set_filter_review_state(&mut self, filter_review_state: ui::FilterReviewState) {
        *self.filter_review_state.borrow_mut() = filter_review_state;
        self.filter_model.reset();
    }

    pub fn sort_model(&self) -> ModelRc<ui::DiffFileItem> {
        self.sort_model.clone().into()
    }
}

impl Default for FileDiffProxyModels {
    fn default() -> Self {
        let model: ModelRc<ui::DiffFileItem> = Rc::new(IdModel::<ui::DiffFileItem>::default()).into();
        let fm: FileDiffFilterModel = Rc::new(model.filter(Box::new(|_| true)));
        FileDiffProxyModels {
            filter_model: fm.clone(),
            filter_text: Rc::new(RefCell::new(SharedString::new())),
            filter_review_state: Rc::new(RefCell::new(ui::FilterReviewState::Unfiltered)),
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }
}

fn extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}
