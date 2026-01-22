use std::cell::RefCell;
use std::rc::Rc;

use crate::ui;

use slint::{FilterModel, MapModel, ModelRc, SharedString, SortModel};

type FilesMapModel = Rc<MapModel<ModelRc<ui::SlintFileDiff>, fn(ui::SlintFileDiff) -> SharedString>>;
type FilesFilterModel = Rc<FilterModel<FilesMapModel, Box<dyn Fn(&SharedString) -> bool>>>;
type SortFilesModel = Rc<SortModel<FilesFilterModel, fn(&SharedString, &SharedString) -> std::cmp::Ordering>>;
pub struct FilesProxyModel {
    _files_map_model: FilesMapModel,
    files_filter_model: FilesFilterModel,
    filter_pattern: Rc<RefCell<SharedString>>,
    files_sort_model: SortFilesModel,
}

impl FilesProxyModel {
    fn map_diff_file(file_diff: ui::SlintFileDiff) -> SharedString {
        file_diff.file_path
    }
    fn sort_files(a: &SharedString, b: &SharedString) -> std::cmp::Ordering {
        a.to_lowercase().cmp(&b.to_lowercase())
    }
    pub fn new(source_model: ModelRc<ui::SlintFileDiff>) -> Self {
        let filter_pattern = Rc::new(RefCell::new(SharedString::new()));
        let map_model = Rc::new(MapModel::new(source_model, Self::map_diff_file as fn(ui::SlintFileDiff) -> SharedString));

        let filter_callback: Box<dyn Fn(&SharedString) -> bool> = Box::new({
            let filter_pattern = filter_pattern.clone();
            move |text: &SharedString| -> bool {
                let pattern = filter_pattern.borrow();
                text.to_lowercase().contains(pattern.to_lowercase().as_str())
            }
        });
        let filter_model = Rc::new(FilterModel::new(map_model.clone(), filter_callback));
        let sort_model = Rc::new(SortModel::new(
            filter_model.clone(),
            Self::sort_files as fn(&SharedString, &SharedString) -> std::cmp::Ordering,
        ));

        Self {
            _files_map_model: map_model,
            files_filter_model: filter_model,
            filter_pattern: filter_pattern.clone(),
            files_sort_model: sort_model,
        }
    }
    pub fn set_filter_pattern(&self, new_filter_pattern: SharedString) {
        *self.filter_pattern.borrow_mut() = new_filter_pattern;
        self.files_filter_model.reset();
    }
    pub fn ui_model(&self) -> ModelRc<SharedString> {
        self.files_sort_model.clone().into()
    }
}
