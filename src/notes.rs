use std::fs::OpenOptions;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::{path::PathBuf, rc::Rc};

use native_dialog::{FileDialog, MessageDialog, MessageType};

use slint::{Model, SharedString, VecModel};

use crate::ui;

const NOTE_FILE_NAME: &str = "notes.txt";

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

fn read_notes_from_file(path: &PathBuf) -> Result<VecModel<ui::NoteItem>, std::io::Error> {
    let todo_model = slint::VecModel::<ui::NoteItem>::default();

    if !path.exists() {
        return Ok(todo_model);
    }

    for line in read_to_string(path)?.lines() {
        let task_result = todo_txt::Task::from_str(line);
        if let Ok(task) = task_result {
            todo_model.push(ui::NoteItem {
                isFixed: task.finished,
                text: task.subject.into(),
            });
        }
    }
    Ok(todo_model)
}

pub struct Notes {
    notes_model: Rc<VecModel<ui::NoteItem>>,
    note_file: PathBuf,
}
impl Notes {
    pub fn new(path_option: Option<&Path>) -> Notes {
        let note_file = match path_option {
            None => PathBuf::new(),
            Some(p) => {
                let file_path = p.join(NOTE_FILE_NAME);
                if file_path.exists() {
                    file_path
                } else {
                    PathBuf::new()
                }
            }
        };

        let model = read_notes_from_file(&note_file).expect("Wrong notes format!");
        Notes {
            notes_model: Rc::new(model),
            note_file: note_file,
        }
    }

    pub fn notes_model(&self) -> Rc<VecModel<ui::NoteItem>> {
        self.notes_model.clone()
    }

    pub fn add_note(&self, text: SharedString) {
        self.notes_model.push(ui::NoteItem { isFixed: false, text: text })
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
