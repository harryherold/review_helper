use std::fs::read_to_string;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{path::PathBuf, rc::Rc};

use slint::ModelRc;
use slint::{Model, SharedString};

use crate::id_model::IdModel;

use crate::ui;

const NOTE_FILE_NAME: &str = "notes.txt";

fn note_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn write_notes_to_file(vec_model: &IdModel<ui::NoteItem>, path: &PathBuf) -> anyhow::Result<()> {
    let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(path)?;
    for item in vec_model.iter() {
        let subject = {
            if item.context.is_empty() {
                item.text.to_string()
            } else {
                format!("{} #{}", item.text.to_string(), item.context.to_string())
            }
        };
        let task = todo_txt::Task {
            subject: subject,
            finished: item.is_fixed,
            ..Default::default()
        };
        write!(file, "{}\n", task.to_string())?;
    }
    Ok(())
}

fn read_notes_from_file(path: &PathBuf) -> Result<IdModel<ui::NoteItem>, std::io::Error> {
    let todo_model = IdModel::<ui::NoteItem>::default();

    if !path.exists() {
        return Ok(todo_model);
    }

    for line in read_to_string(path)?.lines() {
        let task_result = todo_txt::Task::from_str(line);
        if let Ok(task) = task_result {
            let (subject, context) = {
                if task.subject.contains("#") {
                    let parts: Vec<&str> = task.subject.split("#").collect();
                    let subject = parts[0].trim().to_string();
                    let context = parts[1].trim().to_string();
                    (subject, context)
                } else {
                    (task.subject.to_string(), "".to_string())
                }
            };
            let id = note_id();
            todo_model.add(id, ui::NoteItem {
                id: id as i32,
                is_fixed: task.finished,
                text: subject.into(),
                context: context.into(),
            });
        }
    }
    Ok(todo_model)
}

pub struct Notes {
    notes_model: Rc<IdModel<ui::NoteItem>>,
    note_file: PathBuf,
}
impl Notes {
    fn notes_file_exists(path: &Path) -> bool {
        path.join(NOTE_FILE_NAME).exists()
    }

    fn from_file(path: &Path) -> anyhow::Result<Notes> {
        let file_path = path.join(NOTE_FILE_NAME);
        Ok(Notes {
            notes_model: Rc::new(read_notes_from_file(&file_path)?),
            note_file: file_path,
        })
    }

    pub fn new(path: &Path) -> anyhow::Result<Notes> {
        if Notes::notes_file_exists(path) {
            Notes::from_file(path)
        } else {
            Ok(Notes {
                notes_model: Rc::new(IdModel::<ui::NoteItem>::default()),
                note_file: path.join(NOTE_FILE_NAME),
            })
        }
    }

    pub fn default() -> Notes {
        Notes {
            notes_model: Rc::new(IdModel::<ui::NoteItem>::default()),
            note_file: "".into(),
        }
    }

    pub fn notes_model(&self) -> ModelRc<ui::NoteItem> {
        self.notes_model.clone().into()
    }

    pub fn add_note(&self, text: SharedString, context: SharedString) {
        let id = note_id();
        self.notes_model.add(id, ui::NoteItem {
            id: id as i32,
            is_fixed: false,
            text: text,
            context: context,
        })
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if self.notes_model.row_count() > 0 {
            write_notes_to_file(&self.notes_model, &self.note_file)
        } else {
            Ok(())
        }
    }

    pub fn toogle_is_fixed(&self, id: usize) {
        if let Some(mut item) = self.notes_model.get(id) {
            item.is_fixed = !item.is_fixed;
            self.notes_model.update(id, item);
        }
    }

    pub fn set_note_text(&self, id: usize, text: SharedString) {
        if let Some(mut item) = self.notes_model.get(id) {
            if item.text == text {
                return;
            }
            item.text = text;
            self.notes_model.update(id, item);
        }
    }

    pub fn delete_note(&mut self, id: usize) {
        self.notes_model.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use std::{env::current_dir, fs::remove_file, path::Path};

    use slint::Model;

    use crate::notes::NOTE_FILE_NAME;

    use super::Notes;

    fn create_dummy_notes(path: &Path) -> Notes {
        let notes_result = Notes::new(path);
        assert!(notes_result.is_ok());
        let notes = notes_result.unwrap();
        notes.add_note("foo".into(), "".into());
        notes
    }
    fn remove_notes_file(path: &Path) {
        let file_path = path.join(NOTE_FILE_NAME);
        let result = remove_file(file_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_notes() {
        let dir = current_dir().expect("Could not determine cwd!");
        let notes = create_dummy_notes(&dir);
        assert_eq!(notes.notes_model().row_count(), 1);
        let note = notes.notes_model.row_data(0);
        assert!(note.is_some());
        let note = note.unwrap();
        assert_eq!(note.text, "foo");
        assert_eq!(note.context, "bar");
        assert_eq!(note.is_fixed, false);
    }

    #[test]
    fn test_save_read_notes() {
        let dir = current_dir().expect("Could not determine cwd!");

        {
            let notes = create_dummy_notes(&dir);
            notes.add_note("baz".into(), "/usr/share/foo.h".into());
            notes.toogle_is_fixed(1);
            let result = notes.save();
            assert!(result.is_ok());
        }
        {
            let notes = Notes::new(&dir);
            assert!(notes.is_ok());
            let notes = notes.unwrap();
            assert_eq!(notes.notes_model().row_count(), 2);
            let note = notes.notes_model.row_data(0);

            assert!(note.is_some() == true);
            let note = note.unwrap();
            assert_eq!(note.text, "foo");
            assert_eq!(note.context, "");
            assert_eq!(note.is_fixed, false);

            let note = notes.notes_model.row_data(1);

            assert!(note.is_some() == true);
            let note = note.unwrap();
            assert_eq!(note.text, "baz");
            assert_eq!(note.context, "/usr/share/foo.h");
            assert_eq!(note.is_fixed, true);
        }
        remove_notes_file(&dir);
    }
}
