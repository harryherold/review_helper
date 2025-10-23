use std::{cell::RefCell, rc::Rc};

use crate::app_state::AppState;
use crate::git_command_spawner::async_query_diff_tools;
use crate::ui;
use slint::{ComponentHandle, SharedString};

pub fn setup_app_config(app_state: Rc<RefCell<AppState>>) {
    app_state.borrow().app_window.global::<ui::AppConfig>().on_save({
        let state = app_state.clone();
        let ui_weak = app_state.borrow().app_window.as_weak();

        move || {
            let ui = ui_weak.unwrap();
            let app_config = &mut state.borrow_mut().app_config;
            let ui_app_config = ui.global::<ui::AppConfig>();

            app_config.config.diff_tool = ui_app_config.get_diff_tool().to_string();
            app_config.config.editor = ui_app_config.get_editor().to_string();
            app_config.config.editor_args = ui_app_config.get_editor_args().split(",").map(|s| s.to_string()).collect();
            app_config.config.color_scheme = ui_app_config.get_color_scheme().to_string();
            if let Err(e) = app_config.save() {
                eprintln!("Errors occurred during app config save: {}", e.to_string());
            }
        }
    });

    app_state
        .borrow()
        .app_window
        .global::<ui::AppConfig>()
        .set_diff_tool(SharedString::from(app_state.borrow().app_config.config.diff_tool.clone()));

    app_state
        .borrow()
        .app_window
        .global::<ui::AppConfig>()
        .set_editor(SharedString::from(app_state.borrow().app_config.config.editor.clone()));

    let editor_args = app_state.borrow().app_config.config.editor_args.join(",");
    app_state
        .borrow()
        .app_window
        .global::<ui::AppConfig>()
        .set_editor_args(SharedString::from(editor_args));

    let color_scheme = SharedString::from(app_state.borrow().app_config.config.color_scheme.clone());

    app_state.borrow().app_window.global::<ui::AppConfig>().set_color_scheme(color_scheme.clone());

    app_state.borrow().app_window.set_config_color_scheme(color_scheme);

    app_state
        .borrow()
        .app_window
        .global::<ui::AppConfig>()
        .set_diff_tool_model(app_state.borrow().app_config.diff_tool_model.clone().into());

    app_state.borrow().app_window.global::<ui::AppConfig>().on_refresh_diff_tool_model({
        let state = app_state.clone();
        move || {
            async_query_diff_tools(state.clone());
        }
    });

    async_query_diff_tools(app_state.clone());
}
