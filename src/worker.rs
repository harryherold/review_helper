use slint::ComponentHandle;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::ui::{self, AppWindow};

pub enum WorkerMessage {
    Quit,
}

pub struct Worker {
    pub channel: UnboundedSender<WorkerMessage>,
    join_handle: std::thread::JoinHandle<()>,
}

impl Worker {
    pub fn new(app_window: &ui::AppWindow) -> Self {
        let (channel, rx) = tokio::sync::mpsc::unbounded_channel();
        let worker_thread = std::thread::spawn({
            let ui_handle = app_window.as_weak();
            move || {
                work_loop(ui_handle, rx);
            }
        });
        Self {
            channel,
            join_handle: worker_thread,
        }
    }
    pub fn join(self) -> std::thread::Result<()> {
        let _ = self.channel.send(WorkerMessage::Quit);
        self.join_handle.join()
    }
}

fn work_loop(ui_weak: slint::Weak<AppWindow>, mut rx: UnboundedReceiver<WorkerMessage>) {
    while let Some(message) = rx.blocking_recv() {
        match message {
            WorkerMessage::Quit => return,
        }
    }
}
