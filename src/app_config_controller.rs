use crate::app_state::AppState;
use crate::ui;
use slint::{ComponentHandle, SharedString};

pub fn setup_app_config(app_state: &AppState) {
    app_state.app_window.global::<ui::AppConfig>().on_save({
        let app_config = app_state.app_config.clone();
        let ui_weak = app_state.app_window.as_weak();

        move || {
            let ui = ui_weak.unwrap();
            let mut app_config = app_config.borrow_mut();
            let ui_app_config = ui.global::<ui::AppConfig>();

            app_config.config.diff_tool = ui_app_config.get_diff_tool().to_string();
            app_config.config.editor = ui_app_config.get_editor().to_string();
            app_config.config.editor_args = ui_app_config.get_editor_args().split(",").map(|s| s.to_string()).collect();

            if let Err(e) = app_config.save() {
                eprintln!("Errors occurred during app config save: {}", e.to_string());
            }
        }
    });

    app_state
        .app_window
        .global::<ui::AppConfig>()
        .set_diff_tool(SharedString::from(app_state.app_config.borrow().config.diff_tool.clone()));

    app_state
        .app_window
        .global::<ui::AppConfig>()
        .set_editor(SharedString::from(app_state.app_config.borrow().config.editor.clone()));

    let editor_args = app_state.app_config.borrow().config.editor_args.join(",");
    app_state.app_window.global::<ui::AppConfig>().set_editor_args(SharedString::from(editor_args));
}
