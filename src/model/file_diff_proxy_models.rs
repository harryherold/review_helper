use crate::ui;
use slint::{FilterModel, ModelRc, SharedString, SortModel};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::path::Path;
use std::rc::Rc;

use wildcard::Wildcard;

type FileDiffFilterModel = Rc<FilterModel<ModelRc<ui::SlintFileDiff>, Box<dyn Fn(&ui::SlintFileDiff) -> bool>>>;
type FileDiffSortModel = Rc<SortModel<FileDiffFilterModel, Box<dyn Fn(&ui::SlintFileDiff, &ui::SlintFileDiff) -> Ordering>>>;
type SortCallback = Box<dyn Fn(&ui::SlintFileDiff, &ui::SlintFileDiff) -> Ordering>;

pub struct FileDiffProxyModels {
    filter_model: FileDiffFilterModel,
    filter_pattern: Rc<RefCell<SharedString>>,
    filter_review_state: Rc<RefCell<ui::SlintFilterReviewState>>,
    sort_model: FileDiffSortModel,
    sort_criteria: Rc<RefCell<ui::SlintSortCriteria>>,
}

impl FileDiffProxyModels {
    fn sort_by_name(lhs: &ui::SlintFileDiff, rhs: &ui::SlintFileDiff) -> Ordering {
        lhs.file_path.to_lowercase().cmp(&rhs.file_path.to_lowercase())
    }
    fn sort_by_extension(lhs: &ui::SlintFileDiff, rhs: &ui::SlintFileDiff) -> Ordering {
        let lhs_opt_ext = extension_from_filename(&lhs.file_path);
        let rhs_opt_ext = extension_from_filename(&rhs.file_path);
        if let Some(lhs_ext) = lhs_opt_ext
            && let Some(rhs_ext) = rhs_opt_ext
        {
            let result = lhs_ext.cmp(rhs_ext);
            if result == Ordering::Equal {
                lhs.file_path.to_lowercase().cmp(&rhs.file_path.to_lowercase())
            } else {
                result
            }
        } else if lhs_opt_ext.is_some() && rhs_opt_ext.is_none() {
            Ordering::Greater
        } else if lhs_opt_ext.is_none() && rhs_opt_ext.is_some() {
            Ordering::Less
        } else {
            lhs.file_path.to_lowercase().cmp(&rhs.file_path.to_lowercase())
        }
    }
    fn sort_by_is_done(lhs: &ui::SlintFileDiff, rhs: &ui::SlintFileDiff) -> Ordering {
        let lhs_is_done = lhs.is_reviewed;
        let rhs_is_done = rhs.is_reviewed;

        if lhs_is_done && !rhs_is_done {
            Ordering::Less
        } else if !lhs_is_done && rhs_is_done {
            Ordering::Greater
        } else {
            lhs.file_path.to_lowercase().cmp(&rhs.file_path.to_lowercase())
        }
    }

    pub fn new(source_model: ModelRc<ui::SlintFileDiff>) -> Self {
        let filter_pattern = Rc::new(RefCell::new(SharedString::new()));
        let filter_review_state = Rc::new(RefCell::new(ui::SlintFilterReviewState::Unfiltered));

        let filter_callback: Box<dyn Fn(&ui::SlintFileDiff) -> bool> = Box::new({
            let filter_pattern = filter_pattern.clone();
            let filter_review_state = filter_review_state.clone();
            move |item: &ui::SlintFileDiff| -> bool {
                let filter_review_state = *filter_review_state.borrow();
                if filter_review_state == ui::SlintFilterReviewState::Done && !item.is_reviewed {
                    return false;
                }
                if filter_review_state == ui::SlintFilterReviewState::Open && item.is_reviewed {
                    return false;
                }
                if filter_pattern.borrow().is_empty() {
                    true
                } else {
                    let pattern_text = filter_pattern.borrow().as_str().to_lowercase();
                    let pattern = Wildcard::new(pattern_text.as_bytes()).expect("Could not build wildcard!");
                    let file_path = item.file_path.to_lowercase();
                    pattern.is_match(file_path.as_bytes())
                }
            }
        });

        let filter_model = Rc::new(FilterModel::new(source_model, filter_callback));

        let sort_criteria = Rc::new(RefCell::new(ui::SlintSortCriteria::Name));

        let sort_callback: SortCallback = Box::new({
            let sort_citeria = sort_criteria.clone();
            move |lhs, rhs| -> Ordering {
                match *sort_citeria.borrow() {
                    ui::SlintSortCriteria::Name => Self::sort_by_name(lhs, rhs),
                    ui::SlintSortCriteria::Extension => Self::sort_by_extension(lhs, rhs),
                    ui::SlintSortCriteria::IsDone => Self::sort_by_is_done(lhs, rhs),
                }
            }
        });

        let sort_model = Rc::new(SortModel::new(filter_model.clone(), sort_callback));

        FileDiffProxyModels {
            filter_model: filter_model.clone(),
            filter_pattern,
            filter_review_state,
            sort_model,
            sort_criteria,
        }
    }

    pub fn set_sort_by(&self, sort_criteria: ui::SlintSortCriteria) {
        *self.sort_criteria.borrow_mut() = sort_criteria;
        self.sort_model.reset();
    }

    pub fn set_filter_pattern(&self, new_filter_pattern: SharedString) {
        *self.filter_pattern.borrow_mut() = new_filter_pattern;
        self.filter_model.reset();
    }

    pub fn set_filter_review_state(&self, filter_review_state: ui::SlintFilterReviewState) {
        *self.filter_review_state.borrow_mut() = filter_review_state;
        self.filter_model.reset();
    }

    pub fn ui_model(&self) -> ModelRc<ui::SlintFileDiff> {
        self.sort_model.clone().into()
    }
}

fn extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}
