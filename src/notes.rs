use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, path::PathBuf, rc::Rc};
use std::collections::BTreeMap;
use slint::ModelRc;
use slint::{Model, SharedString};

use crate::id_model::{IdModel, IdModelChange};

use crate::ui;

const NOTE_FILE_NAME: &str = "notes.md";

fn note_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn read_notes_from_file(path: &PathBuf) -> anyhow::Result<IdModel<ui::NoteItem>> {
    let to_note = |line: &str| -> Option<(bool, String)> {
        let pos = line.find("[")?;
        let is_fixed = false == line.get(pos + 1..)?.starts_with("]");
        let text: String = if is_fixed {
            let pos = line.find("]")?;
            line.get(pos + 1..)?.trim().to_string()
        } else {
            line.get(pos + 2..)?.trim().to_string()
        };
        Some((is_fixed, text))
    };
    let to_file = |line: &str| -> Option<String> {
        let start = line.find("'")? + 1;
        let end = line.rfind("'")?;
        Some(line.get(start..end)?.to_string())
    };
    let model = IdModel::<ui::NoteItem>::default();
    let buffer = fs::read_to_string(path)?;
    let mut iter = buffer.lines().into_iter();
    let mut context = String::new();

    while let Some(line) = iter.next() {
        let line = line.trim();
        if line.starts_with("#") {
            context = to_file(line).expect("Error while parsing heading");
        } else if line.starts_with("*") {
            let (is_fixed, text) = to_note(line).expect("Error while parsing ListItem");
            let id = note_id();
            model.add(
                id,
                ui::NoteItem {
                    id: id as i32,
                    is_fixed,
                    text: text.into(),
                    context: context.clone().into(),
                },
            );
        }
    }
    anyhow::Ok(model)
}
fn write_notes_to_file(model: &IdModel<ui::NoteItem>, path: &PathBuf) -> anyhow::Result<()> {
    let mut general_notes = Vec::<String>::new();
    let mut file_notes = BTreeMap::<String, Vec<String>>::new();

    let note_item_to_string = |item: &ui::NoteItem| -> String {
        format!("* [{}] {}", if item.is_fixed { "x" } else { "" }, item.text)
    };

    for item in model.iter() {
        let notes: &mut Vec<String> = if item.context.is_empty() {
            &mut general_notes
        } else {
            file_notes.entry(item.context.to_string()).or_insert(Vec::new())
        };
        notes.push(note_item_to_string(&item));
    }
    let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(path)?;

    for note in general_notes {
        write!(file, "{}\n", note)?;
    }

    write!(file, "\n")?;

    for (file_name, notes) in file_notes {
        write!(file, "# Notes of '{}'\n", file_name)?;
        for note in notes {
            write!(file, "{}\n", note)?;
        }
        write!(file, "\n")?;
    }

    anyhow::Ok(())
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

    pub fn toggle_is_fixed(&self, id: usize) {
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

    pub fn observe_notes_model<Observer: Fn(IdModelChange) + 'static>(&self, observer: Observer) {
        self.notes_model.set_observer(observer);
    }
}

#[cfg(test)]
mod tests {
    use std::{env, fs, path::PathBuf};
    use slint::{Model, SharedString};

    use crate::{id_model::IdModel, ui};

    use super::{read_notes_from_file, write_notes_to_file, Notes};

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

    fn assert_eq_notes(notes: &IdModel<ui::NoteItem>, expected_notes: &IdModel<ui::NoteItem>) {
        assert_eq!(notes.row_count(), expected_notes.row_count());
        for i in 0..notes.row_count() {
            let note = notes.row_data(i);
            assert!(note.is_some());
            let note = note.unwrap();

            let expected_note = expected_notes.row_data(i);
            assert!(expected_note.is_some());
            let expected_note = expected_note.unwrap();

            assert_eq!(note.text, expected_note.text);
            assert_eq!(note.context, expected_note.context);
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

    #[test]
    fn test_read_markdown() {
        let mut path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"));
        path.push("docs");
        path.push("foo.md");
        let notes = read_notes_from_file(&path);
        assert!(notes.is_ok());
        let notes = notes.unwrap();
        assert_eq!(notes.row_count(), 5);
        let expected_notes = IdModel::<ui::NoteItem>::default();
        expected_notes.add(1, ui::NoteItem{
            id: 1,
            is_fixed: false,
            text: "foo".into(),
            context: "".into(),
        });
        expected_notes.add(2, ui::NoteItem{
            id: 2,
            is_fixed: false,
            text: "dasdas".into(),
            context: "".into(),
        });
        expected_notes.add(3, ui::NoteItem{
            id: 3,
            is_fixed: true,
            text: "foo bar".into(),
            context: "/tmp/foo.c".into(),
        });
        expected_notes.add(4, ui::NoteItem{
            id: 4,
            is_fixed: true,
            text: "flupp bubb".into(),
            context: "/tmp/foo.c".into(),
        });
        expected_notes.add(5, ui::NoteItem{
            id: 5,
            is_fixed: true,
            text: "schupp".into(),
            context: "C:\\blubb\\bar.cpp".into(),
        });
        assert_eq_notes(&notes, &expected_notes);
    }

    #[test]
    fn test_write_markdown() {
        let mut path = test_dir_path();
        if !path.exists() {
            assert!(fs::create_dir(&path).is_ok());
        }
        path.push("foo.md");

        let expected_notes = IdModel::<ui::NoteItem>::default();
        expected_notes.add(1, ui::NoteItem{
            id: 1,
            is_fixed: false,
            text: "foo".into(),
            context: "".into(),
        });
        expected_notes.add(2, ui::NoteItem{
            id: 2,
            is_fixed: false,
            text: "dasdas".into(),
            context: "".into(),
        });
        expected_notes.add(3, ui::NoteItem{
            id: 3,
            is_fixed: true,
            text: "schupp".into(),
            context: "C:\\blubb\\bar.cpp".into(),
        });
        expected_notes.add(4, ui::NoteItem{
            id: 4,
            is_fixed: true,
            text: "foo bar".into(),
            context: "C:\\blubb\\foo.cpp".into(),
        });
        expected_notes.add(5, ui::NoteItem{
            id: 5,
            is_fixed: false,
            text: "flupp bubb".into(),
            context: "C:\\blubb\\foo.cpp".into(),
        });
        assert!(write_notes_to_file(&expected_notes, &path).is_ok());

        let read_notes = read_notes_from_file(&path);
        assert!(read_notes.is_ok());
        let read_notes = read_notes.unwrap();

        assert_eq_notes(&read_notes, &expected_notes);
        
        if path.exists() {
            assert!(fs::remove_file(&path).is_ok());
        }
    }
}
