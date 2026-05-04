use slint::ComponentHandle;

use crate::ui::{self, AppWindow};
use crate::unwrap_or_return;

use crate::worker::WorkerChannel;

pub fn setup_review_helper_settings(app_window: &AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintReviewHelperSettings>().on_save({
        let ui_weak = app_window.as_weak();
        let channel = worker_channel.clone();

        move || {
            let ui = unwrap_or_return!(ui_weak.upgrade(), "Upgrade to AppWindow failed!");
            let ui_app_config = ui.global::<ui::SlintReviewHelperSettings>();

            let diff_tool = ui_app_config.get_diff_tool().to_string();
            let editor = ui_app_config.get_editor().to_string();
            let editor_args = ui_app_config.get_editor_args().split(",").map(|s| s.to_string()).collect();
            let color_scheme = ui_app_config.get_color_scheme().to_string();
            channel
                .send(crate::worker::WorkerMessage::SaveReviewHelperSettings {
                    diff_tool,
                    editor,
                    editor_args,
                    color_scheme,
                })
                .expect("Worker channel broken!");
        }
    });

    app_window.global::<ui::SlintReviewHelperSettings>().on_refresh_diff_tool_model({
        let channel = worker_channel.clone();
        move || {
            channel.send(crate::worker::WorkerMessage::QueryDiffTools).expect("Worker channel broken!");
        }
    });

    worker_channel
        .send(crate::worker::WorkerMessage::QueryDiffTools)
        .expect("Worker channel broken!");
}
