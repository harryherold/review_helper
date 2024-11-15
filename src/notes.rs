use std::fs::read_to_string;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::{path::PathBuf, rc::Rc};

use slint::{Model, SharedString, VecModel};

use crate::ui;

const NOTE_FILE_NAME: &str = "notes.txt";

fn write_notes_to_file(vec_model: &VecModel<ui::NoteItem>, path: &PathBuf) -> anyhow::Result<()> {
    let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(path)?;
    for item in vec_model.iter() {
        let task = todo_txt::Task {
            subject: item.text.to_string(),
            finished: item.is_fixed,
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
                is_fixed: task.finished,
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
    fn notes_file_exists(path: &Path) -> bool {
        path.join(NOTE_FILE_NAME).exists()
    }

    pub fn new(path: &Path) -> anyhow::Result<Notes> {
        if Notes::notes_file_exists(path) {
            Notes::from_file(path)
        } else {
            Ok(Notes {
                notes_model: Rc::new(slint::VecModel::<ui::NoteItem>::default()),
                note_file: path.join(NOTE_FILE_NAME),
            })
        }
    }

    pub fn default() -> Notes {
        Notes {
            notes_model: Rc::new(slint::VecModel::<ui::NoteItem>::default()),
            note_file: "".into(),
        }
    }

    fn from_file(path: &Path) -> anyhow::Result<Notes> {
        let file_path = path.join(NOTE_FILE_NAME);
        Ok(Notes {
            notes_model: Rc::new(read_notes_from_file(&file_path)?),
            note_file: file_path,
        })
    }

    pub fn notes_model(&self) -> Rc<VecModel<ui::NoteItem>> {
        self.notes_model.clone()
    }

    pub fn add_note(&self, text: SharedString) {
        self.notes_model.push(ui::NoteItem { is_fixed: false, text: text })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if self.notes_model.row_count() > 0 {
            write_notes_to_file(&self.notes_model, &self.note_file)
        } else {
            Ok(())
        }
    }

    pub fn toogle_is_fixed(&self, note_index: usize) {
        if let Some(mut item) = self.notes_model.row_data(note_index) {
            item.is_fixed = !item.is_fixed;
            self.notes_model.set_row_data(note_index, item);
        }
    }

    pub fn set_note_text(&self, note_index: usize, text: SharedString) {
        if let Some(mut item) = self.notes_model.row_data(note_index) {
            if item.text == text {
                return;
            }
            item.text = text;
            self.notes_model.set_row_data(note_index, item);
        }
    }
}
