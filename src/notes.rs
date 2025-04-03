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
            todo_model.add(
                id,
                ui::NoteItem {
                    id: id as i32,
                    is_fixed: task.finished,
                    text: subject.into(),
                    context: context.into(),
                },
            );
        }
    }
    Ok(todo_model)
}

pub struct Notes {
    notes_model: Rc<IdModel<ui::NoteItem>>,
    note_file: PathBuf,
}

impl Default for Notes {
    fn default() -> Self {
        Notes {
            notes_model: Rc::new(IdModel::<ui::NoteItem>::default()),
            note_file: "".into(),
        }
    }
}

impl Notes {
    pub fn new(path: &Path) -> anyhow::Result<Notes> {
        let note_file = path.join(NOTE_FILE_NAME);
        if note_file.exists() {
            Ok(Notes {
                notes_model: Rc::new(read_notes_from_file(&note_file)?),
                note_file,
            })
        } else {
            Ok(Notes {
                notes_model: Rc::new(IdModel::<ui::NoteItem>::default()),
                note_file,
            })
        }
    }

    pub fn notes_model(&self) -> ModelRc<ui::NoteItem> {
        self.notes_model.clone().into()
    }

    pub fn add_note(&self, text: SharedString, context: SharedString) {
        let id = note_id();
        self.notes_model.add(
            id,
            ui::NoteItem {
                id: id as i32,
                is_fixed: false,
                text,
                context,
            },
        )
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
    use std::{env, fs, path::PathBuf};

    use slint::{Model, SharedString};

    use super::Notes;

    struct TestContext {
        notes: Notes,
        path: PathBuf,
        is_clean_enabled: bool,
    }

    impl Drop for TestContext {
        fn drop(&mut self) {
            if self.is_clean_enabled {
                let result = fs::remove_dir_all(&self.path);
                assert!(result.is_ok());
            }
        }
    }

    fn test_dir_path() -> PathBuf {
        let mut path = env::temp_dir();
        let mut app_name = std::env!("CARGO_CRATE_NAME").to_string();
        app_name.push_str("_notes_tests");
        path.push(app_name);
        path
    }

    fn setup(is_clean_enabled: bool) -> TestContext {
        let path = test_dir_path();
        if !path.exists() {
            assert!(fs::create_dir(&path).is_ok());
        }
        let notes = Notes::new(&path);
        assert!(notes.is_ok());
        TestContext {
            notes: notes.unwrap(),
            path,
            is_clean_enabled,
        }
    }

    #[test]
    fn test_add_notes() {
        {
            let ctx = setup(false);
            ctx.notes.add_note("foo".into(), "/tmp/test.cpp".into());
            ctx.notes.add_note("bar".into(), "/tmp/test.h".into());
            assert!(ctx.notes.save().is_ok());
        }
        {
            let ctx = setup(true);
            let model = ctx.notes.notes_model();

            assert_eq!(model.row_count(), 2);

            let note = model.row_data(0);
            assert!(note.is_some());

            let note = note.unwrap();
            assert_eq!(note.text, Into::<SharedString>::into("foo"));
            assert_eq!(note.context, Into::<SharedString>::into("/tmp/test.cpp"));

            let note = model.row_data(1);
            assert!(note.is_some());

            let note = note.unwrap();
            assert_eq!(note.text, Into::<SharedString>::into("bar"));
            assert_eq!(note.context, Into::<SharedString>::into("/tmp/test.h"));
        }
    }
}
