use std::fs::OpenOptions;
use std::fs::{read_to_string, File};
use std::io::{Error, Write};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::{path::PathBuf, rc::Rc};

use native_dialog::{FileDialog, MessageDialog, MessageType};

use slint::{Model, SharedString, VecModel};

slint::include_modules!();

fn write_todos_to_file(vec_model: &VecModel<ReviewTodoItem>, path: &PathBuf, should_create_file: bool) -> Result<(), Error> {
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

fn read_todos_from_file(todo_model: &VecModel<ReviewTodoItem>, path: &PathBuf) -> Result<(), Error> {
    for line in read_to_string(path)?.lines() {
        let task_result = todo_txt::Task::from_str(line);
        if let Ok(task) = task_result {
            todo_model.push(ReviewTodoItem {
                isFixed: task.finished,
                text: task.subject.into(),
            });
        } else {
            ()
        }
    }
    Ok(())
}

fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) {
    let args = ["diff", "--name-only", start_commit, end_commit];
    let output = Command::new("git").current_dir(repo_path).args(args).output().expect("git diff failed!");
    println!("{}", String::from_utf8(output.stdout).unwrap());
}

pub struct Review {
    todo_model: Rc<VecModel<ReviewTodoItem>>,
    file_diff_model: Rc<VecModel<ReviewFileItem>>,
    todo_file: PathBuf,
    repo_path: PathBuf,
}
impl Review {
    pub fn new() -> Review {
        Review {
            todo_model: Rc::new(slint::VecModel::<ReviewTodoItem>::default()),
            file_diff_model: Rc::new(slint::VecModel::<ReviewFileItem>::default()),
            todo_file: PathBuf::new(),
            repo_path: PathBuf::new(),
        }
    }

    pub fn todo_model(&self) -> Rc<VecModel<ReviewTodoItem>> {
        self.todo_model.clone()
    }

    pub fn file_diff_model(&self) -> Rc<VecModel<ReviewFileItem>> {
        self.file_diff_model.clone()
    }

    pub fn add_todo(&self, text: SharedString) {
        self.todo_model.push(ReviewTodoItem { isFixed: false, text: text })
    }

    pub fn open_todos(&mut self) -> Option<SharedString> {
        // self.todo_file = PathBuf::new();
        if self.todo_file.as_os_str().is_empty() {
            self.todo_file = FileDialog::new()
                .set_location("~")
                .add_filter("Text File (*.txt)", &["txt"])
                .show_open_single_file()
                .unwrap()?;
        }
        let result = read_todos_from_file(&self.todo_model, &self.todo_file);
        if result.is_err() {
            let _ = MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Error")
                .set_text("Errors occured during reading file!")
                .show_alert();
            return None;
        }
        Some(SharedString::from(self.todo_file.to_str()?))
    }

    pub fn save_todos(&mut self) -> Option<SharedString> {
        let should_create_file = self.todo_file.as_os_str().is_empty();
        if should_create_file {
            let path = FileDialog::new()
                .set_location("~")
                .add_filter("Text File (*.txt)", &["txt"])
                .show_save_single_file()
                .unwrap()?;
            self.todo_file = path;
        }
        let result = write_todos_to_file(&self.todo_model, &self.todo_file, should_create_file);
        if let Err(_) = result {
            let _r = MessageDialog::new()
                .set_type(MessageType::Error)
                .set_title("Abort")
                .set_text("Could save comments!")
                .show_alert();
        }
        Some(SharedString::from(self.todo_file.to_str()?))
    }

    pub fn toogle_is_fixed(&self, todo_index: usize) {
        let data = self.todo_model.row_data_tracked(todo_index);
        if let Some(item) = data {
            self.todo_model.set_row_data(
                todo_index as usize,
                ReviewTodoItem {
                    isFixed: !item.isFixed,
                    text: item.text,
                },
            );
        }
    }

    pub fn set_todo_text(&self, todo_index: usize, text: SharedString) {
        let data = self.todo_model.row_data_tracked(todo_index);
        if let Some(item) = data {
            if item.text != text {
                self.todo_model.set_row_data(
                    todo_index,
                    ReviewTodoItem {
                        isFixed: item.isFixed,
                        text: text,
                    },
                );
            }
        }
    }

    pub fn open_repo(&mut self) -> Option<SharedString> {
        if self.repo_path.as_os_str().is_empty() {
            self.repo_path = FileDialog::new().set_location("~/workspace/review-todo").show_open_single_dir().unwrap()?;
        }
        Some(SharedString::from(self.repo_path.to_str()?))
    }

    pub fn diff_repo(&mut self, start_commit: SharedString, end_commit: SharedString) {
        let args = ["diff", "--name-only", start_commit.as_str(), end_commit.as_str()];
        let output = Command::new("git").current_dir(&self.repo_path).args(args).output().expect("git diff failed!");
        println!("Diff!");
        // println!("{}", String::from_utf8(output.stdout).unwrap());
    }
}
