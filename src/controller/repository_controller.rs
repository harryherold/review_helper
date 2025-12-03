use crate::{storage::RepositoryName, ui, worker::WorkerChannel};

use slint::ComponentHandle;

pub fn setup_repository_callbacks(app_window: &ui::AppWindow, worker_channel: WorkerChannel) {
    app_window.global::<ui::SlintRepositoryCallbacks>().on_repository_changed({
        let channel = worker_channel.clone();
        move |repository_name, base_branch| {
            let name = RepositoryName::from(repository_name.as_str());
            let base_branch = String::from(base_branch);
            channel.send(crate::worker::WorkerMessage::ChangeRepository { name, base_branch }).unwrap();
        }
    });
    app_window.global::<ui::SlintRepositoryCallbacks>().on_load_repository({
        let channel = worker_channel.clone();
        move |id, name| {
            let name = RepositoryName::from(name.as_str());
            channel.send(crate::worker::WorkerMessage::LoadReviewNames { id: id as usize, name }).unwrap();
        }
    });
}
