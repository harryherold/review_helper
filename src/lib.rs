use std::fs::OpenOptions;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::process::Command;
use std::str::FromStr;
use std::{path::PathBuf, rc::Rc};

use native_dialog::{FileDialog, MessageDialog, MessageType};

use anyhow::Result;

use slint::{Model, SharedString, VecModel};

slint::include_modules!();

fn write_todos_to_file(vec_model: &VecModel<ReviewTodoItem>, path: &PathBuf, should_create_file: bool) -> Result<(), std::io::Error> {
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

fn read_todos_from_file(todo_model: &VecModel<ReviewTodoItem>, path: &PathBuf) -> Result<(), std::io::Error> {
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

fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> Result<String> {
    let mut args = vec!["diff", "--name-only"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = Command::new("git").current_dir(repo_path).args(args).output()?;

    String::from_utf8(output.stdout).map_err(|e| anyhow::Error::from(e))
}

fn diff_file(repo_path: &PathBuf, start_commit: &str, end_commit: &str, file: &str) -> Result<()> {
    let mut args = vec!["difftool", "-U100000", "--no-prompt", "--tool=meld"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    args.push(file);

    Command::new("git").current_dir(repo_path).args(args).spawn()?;
    Ok(())
}

pub struct Review {
    todo_model: Rc<VecModel<ReviewTodoItem>>,
    todo_file: PathBuf,
    repo_path: PathBuf,
    // TODO refactor: move in on diff struct
    start_commit: String,
    end_commit: String,
    file_diff_model: Rc<VecModel<ReviewFileItem>>,
}
impl Review {
    pub fn new() -> Review {
        Review {
            todo_model: Rc::new(slint::VecModel::<ReviewTodoItem>::default()),
            todo_file: PathBuf::new(),
            repo_path: PathBuf::new(),
            start_commit: String::new(),
            end_commit: String::new(),
            file_diff_model: Rc::new(slint::VecModel::<ReviewFileItem>::default()),
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
        self.file_diff_model.clear();

        self.start_commit = start_commit.to_string();
        self.end_commit = end_commit.to_string();

        let diff_result = diff_git_repo(&self.repo_path, &self.start_commit, &self.end_commit);
        if let Err(e) = diff_result {
            // TODO proper error handling
            eprintln!("Diff of repo failed: {}", e.to_string());
            return;
        }
        let output_text = diff_result.unwrap();
        output_text.split('\n').filter(|file| false == file.is_empty()).for_each(|file| {
            self.file_diff_model.push(ReviewFileItem {
                text: file.into(),
                isReviewed: false,
            })
        });
    }

    pub fn diff_file(&self, index: i32) {
        match self.file_diff_model.row_data(index as usize) {
            None => eprintln!("Could not found file!"), // TODO proper error handling
            Some(file_item) => {
                if let Err(e) = diff_file(&self.repo_path, &self.start_commit, &self.end_commit, &file_item.text) {
                    // TODO proper error handling
                    eprintln!("File diff failed: {}", e.to_string());
                }
            }
        }
    }
}
