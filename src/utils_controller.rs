use crate::app_state::AppState;
use crate::ui;
use slint::ComponentHandle;
use std::path::PathBuf;

pub fn setup_utils(app_state: &AppState) {
    app_state.app_window.global::<ui::StringUtils>().on_filename({
        |path| {
            if let Some(file_name) = PathBuf::from(path.to_string()).file_name() {
                file_name.to_str().expect("Could not parse os string!").to_string().into()
            } else {
                "".into()
            }
        }
    });
}
