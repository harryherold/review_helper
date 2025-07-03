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

    pub fn new(model: ModelRc<ui::DiffFileItem>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();

        let fm: FileDiffFilterModel = Rc::new(FilterModel::new(
            model,
            Box::new(move |item: &ui::DiffFileItem| -> bool {
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
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }

    pub fn sort_by(&mut self, sort_criteria: ui::SortCriteria) {
        if sort_criteria == ui::SortCriteria::Name {
            self.sort_model = Rc::new(self.filter_model.clone().sort_by(Self::sort_by_name));
        } else {
            self.sort_model = Rc::new(self.filter_model.clone().sort_by(Self::sort_by_extension));
        }
    }

    pub fn set_filter_text(&mut self, filter_text: SharedString) {
        *self.filter_text.borrow_mut() = filter_text;
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
            sort_model: Rc::new(fm.sort_by(Self::sort_by_name)),
        }
    }
}

fn extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}
