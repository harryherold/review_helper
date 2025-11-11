use std::{cell::RefCell, rc::Rc};

use crate::git_command_spawner::async_query_diff_tools;
use crate::model::AppState;
use crate::ui;
use slint::{ComponentHandle, SharedString};

pub fn setup_review_helper_settings(app_state: Rc<RefCell<AppState>>) {
    app_state.borrow().app_window.global::<ui::SlintReviewHelperSettings>().on_save({
        let state = app_state.clone();
        let ui_weak = app_state.borrow().app_window.as_weak();

        move || {
            let ui = ui_weak.unwrap();
            let review_helper_settings = &mut state.borrow_mut().review_helper_settings;
            let ui_app_config = ui.global::<ui::SlintReviewHelperSettings>();

            review_helper_settings.diff_tool = ui_app_config.get_diff_tool().to_string();
            review_helper_settings.editor = ui_app_config.get_editor().to_string();
            review_helper_settings.editor_args = ui_app_config.get_editor_args().split(",").map(|s| s.to_string()).collect();
            review_helper_settings.color_scheme = ui_app_config.get_color_scheme().to_string();
            if let Err(e) = review_helper_settings.save() {
                eprintln!("Errors occurred during app config save: {}", e.to_string());
            }
        }
    });

    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .set_diff_tool(SharedString::from(app_state.borrow().review_helper_settings.diff_tool.clone()));

    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .set_editor(SharedString::from(app_state.borrow().review_helper_settings.editor.clone()));

    let editor_args = app_state.borrow().review_helper_settings.editor_args.join(",");
    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .set_editor_args(SharedString::from(editor_args));

    let color_scheme = SharedString::from(app_state.borrow().review_helper_settings.color_scheme.clone());

    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .set_color_scheme(color_scheme.clone());

    app_state.borrow().app_window.set_config_color_scheme(color_scheme);

    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .set_diff_tool_model(app_state.borrow().review_helper_settings.diff_tool_model.clone().into());

    app_state
        .borrow()
        .app_window
        .global::<ui::SlintReviewHelperSettings>()
        .on_refresh_diff_tool_model({
            let state = app_state.clone();
            move || {
                async_query_diff_tools(state.clone());
            }
        });

    async_query_diff_tools(app_state.clone());
}
