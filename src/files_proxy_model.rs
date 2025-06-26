use std::cell::RefCell;
use std::rc::Rc;

use crate::ui;

use crate::id_model::IdModel;
use slint::{FilterModel, MapModel, ModelRc, SharedString, SortModel};

type FilesMapModel = Rc<MapModel<ModelRc<ui::DiffFileItem>, fn(ui::DiffFileItem) -> SharedString>>;
type FilesFilterModel = Rc<FilterModel<FilesMapModel, Box<dyn Fn(&SharedString) -> bool>>>;
type SortFilesModel = Rc<SortModel<FilesFilterModel, fn(&SharedString, &SharedString) -> std::cmp::Ordering>>;
pub struct FilesProxyModel {
    _files_map_model: FilesMapModel,
    files_filter_model: FilesFilterModel,
    filter_text: Rc<RefCell<SharedString>>,
    files_sort_model: SortFilesModel,
}

impl FilesProxyModel {
    fn map_diff_file_item(file_item: ui::DiffFileItem) -> SharedString {
        file_item.text
    }
    fn sort_files(a: &SharedString, b: &SharedString) -> std::cmp::Ordering {
        a.to_lowercase().cmp(&b.to_lowercase())
    }
    pub fn new(model: ModelRc<ui::DiffFileItem>) -> Self {
        let filter_text = Rc::new(RefCell::new(SharedString::new()));
        let clone_filter_text = filter_text.clone();
        let map_model = Rc::new(MapModel::new(model, Self::map_diff_file_item as fn(ui::DiffFileItem) -> SharedString));

        let filter_callback: Box<dyn Fn(&SharedString) -> bool> = Box::new(move |text: &SharedString| -> bool {
            let filter_text = filter_text.clone();
            let pattern = filter_text.borrow();
            text.to_lowercase().contains(pattern.to_lowercase().as_str())
        });
        let filter_model = FilterModel::new(map_model.clone(), filter_callback);

        let rc_filter_model = Rc::new(filter_model);
        let sort_model = Rc::new(SortModel::new(
            rc_filter_model.clone(),
            Self::sort_files as fn(&SharedString, &SharedString) -> std::cmp::Ordering,
        ));

        Self {
            _files_map_model: map_model,
            files_filter_model: rc_filter_model,
            filter_text: clone_filter_text,
            files_sort_model: sort_model,
        }
    }
    pub fn set_filter_text(&mut self, filter_text: SharedString) {
        *self.filter_text.borrow_mut() = filter_text;
        self.files_filter_model.reset();
    }
    pub fn files_sort_model(&self) -> ModelRc<SharedString> {
        self.files_sort_model.clone().into()
    }
}

impl Default for FilesProxyModel {
    fn default() -> Self {
        let model: ModelRc<ui::DiffFileItem> = Rc::new(IdModel::<ui::DiffFileItem>::default()).into();
        Self::new(model)
    }
}
