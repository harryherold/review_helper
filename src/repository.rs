use std::{path::PathBuf, process::Command, rc::Rc};

use native_dialog::FileDialog;
use slint::{Model, SharedString, VecModel};

use anyhow::Result;

use crate::ui;

pub struct Repository {
    path: PathBuf,
    current_diff: Diff,
}

struct Diff {
    start_commit: String,
    end_commit: String,
    file_diff_model: Rc<VecModel<ui::DiffFileItem>>,
}

impl Diff {
    pub fn new() -> Diff {
        Diff {
            start_commit: String::new(),
            end_commit: String::new(),
            file_diff_model: Rc::new(slint::VecModel::<ui::DiffFileItem>::default()),
        }
    }
}

impl Repository {
    pub fn new() -> Repository {
        Repository {
            path: PathBuf::new(),
            current_diff: Diff::new(),
        }
    }
    // TODO replace SharedString
    pub fn open(&mut self) -> SharedString {
        let repo_path = FileDialog::new().set_location("~/workspace/review-todo").show_open_single_dir().unwrap();

        match repo_path {
            None => SharedString::new(),
            Some(path) => {
                self.path = path;
                SharedString::from(self.path.to_str().unwrap())
            }
        }
    }
    pub fn diff_repository(&mut self, start_commit: &str, end_commit: &str) {
        self.current_diff.start_commit = start_commit.to_string();
        self.current_diff.end_commit = end_commit.to_string();

        self.current_diff.file_diff_model.clear();

        let diff_result = diff_git_repo(&self.path, &start_commit, &end_commit);
        if let Err(e) = diff_result {
            // TODO proper error handling
            eprintln!("Diff of repo failed: {}", e.to_string());
            return;
        }
        let output_text = diff_result.unwrap();
        output_text.split('\n').filter(|file| false == file.is_empty()).for_each(|file| {
            self.current_diff.file_diff_model.push(ui::DiffFileItem {
                text: file.into(),
                isReviewed: false,
            })
        });
    }
    pub fn diff_file(&self, index: i32) {
        match self.current_diff.file_diff_model.row_data(index as usize) {
            None => eprintln!("Could not found file!"), // TODO proper error handling
            Some(file_item) => {
                if let Err(e) = diff_file(&self.path, &self.current_diff.start_commit, &self.current_diff.end_commit, &file_item.text) {
                    // TODO proper error handling
                    eprintln!("File diff failed: {}", e.to_string());
                }
            }
        }
    }
    pub fn file_diff_model(&self) -> Rc<VecModel<ui::DiffFileItem>> {
        self.current_diff.file_diff_model.clone()
    }
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
