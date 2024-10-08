use std::fs::OpenOptions;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::str::FromStr;
use std::{path::PathBuf, rc::Rc};

use native_dialog::{FileDialog, MessageDialog, MessageType};

use slint::{Model, SharedString, VecModel};

use crate::ui;

fn write_notes_to_file(vec_model: &VecModel<ui::NoteItem>, path: &PathBuf, should_create_file: bool) -> Result<(), std::io::Error> {
    let get_file = || {
        if should_create_file {
            return File::create(path);
        } else {
            return OpenOptions::new().write(true).open(path);
        }
    };
    let mut file = get_file()?;
    for item in vec_model.iter() {
        let task = todo_txt::Task {
            subject: item.text.to_string(),
            finished: item.isFixed,
            ..Default::default()
        };
        write!(file, "{}\n", task.to_string())?;
    }
    Ok(())
}

fn read_notes_from_file(todo_model: &VecModel<ui::NoteItem>, path: &PathBuf) -> Result<(), std::io::Error> {
    todo_model.clear();
    for line in read_to_string(path)?.lines() {
        let task_result = todo_txt::Task::from_str(line);
        if let Ok(task) = task_result {
            todo_model.push(ui::NoteItem {
                isFixed: task.finished,
                text: task.subject.into(),
            });
        } else {
            ()
        }
    }
    Ok(())
}

pub struct Notes {
    notes_model: Rc<VecModel<ui::NoteItem>>,
    note_file: PathBuf,
}
impl Notes {
    pub fn new() -> Notes {
        Notes {
            notes_model: Rc::new(slint::VecModel::<ui::NoteItem>::default()),
            note_file: PathBuf::new(),
        }
    }

    pub fn notes_model(&self) -> Rc<VecModel<ui::NoteItem>> {
        self.notes_model.clone()
    }

    pub fn add_note(&self, text: SharedString) {
        self.notes_model.push(ui::NoteItem { isFixed: false, text: text })
    }

    pub fn open(&mut self) -> Option<SharedString> {
        self.note_file = FileDialog::new()
            .set_location("~")
            .add_filter("Text File (*.txt)", &["txt"])
            .show_open_single_file()
            .unwrap()?;

        let result = read_notes_from_file(&self.notes_model, &self.note_file);
        if result.is_err() {
            let _ = MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Error")
                .set_text("Errors occured during reading file!")
                .show_alert();
            return None;
        }
        Some(SharedString::from(self.note_file.to_str()?))
    }

    pub fn save(&mut self) -> Option<SharedString> {
        let should_create_file = self.note_file.as_os_str().is_empty();
        if should_create_file {
            let path = FileDialog::new()
                .set_location("~")
                .add_filter("Text File (*.txt)", &["txt"])
                .show_save_single_file()
                .unwrap()?;
            self.note_file = path;
        }
        let result = write_notes_to_file(&self.notes_model, &self.note_file, should_create_file);
        if let Err(_) = result {
            let _r = MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Abort")
                .set_text("Could save comments!")
                .show_alert();
        }
        Some(SharedString::from(self.note_file.to_str()?))
    }

    pub fn toogle_is_fixed(&self, note_index: usize) {
        let data = self.notes_model.row_data(note_index);
        if let Some(item) = data {
            self.notes_model.set_row_data(
                note_index as usize,
                ui::NoteItem {
                    isFixed: !item.isFixed,
                    text: item.text,
                },
            );
        }
    }

    pub fn set_note_text(&self, note_index: usize, text: SharedString) {
        let data = self.notes_model.row_data(note_index);
        if let Some(item) = data {
            if item.text != text {
                self.notes_model.set_row_data(
                    note_index,
                    ui::NoteItem {
                        isFixed: item.isFixed,
                        text: text,
                    },
                );
            }
        }
    }
}
