use crate::model::AppState;
use crate::ui;
use slint::{ComponentHandle, Model};
use std::{cell::RefCell, path::PathBuf, rc::Rc};

pub fn setup_utils(app_state: Rc<RefCell<AppState>>) {
    app_state.borrow().app_window.global::<ui::SlintStringUtils>().on_filename({
        |path| {
            if let Some(file_name) = PathBuf::from(path.to_string()).file_name() {
                file_name.to_str().expect("Could not parse os string!").to_string().into()
            } else {
                "".into()
            }
        }
    });
    app_state.borrow().app_window.global::<ui::SlintModelUtils>().on_index_of_string({
        |model, value| match model.iter().position(|v| value == v) {
            None => -1,
            Some(i) => i as i32,
        }
    });
    
}
