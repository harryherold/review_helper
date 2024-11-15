use std::{path::PathBuf, rc::Rc};

use native_dialog::FileDialog;
use slint::{Model, VecModel};

use anyhow::Result;

use crate::{config::Config, git_utils, ui};

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
            path: "".into(),
            current_diff: Diff::new(),
        }
    }

    pub fn from_config(config: &Config) -> Repository {
        let mut repo = Repository {
            path: PathBuf::from(config.repo_path.to_string()),
            current_diff: Diff::new(),
        };
        repo.diff_repository(&config.start_diff, &config.end_diff);
        repo
    }

    pub fn is_repo_valid(path: &PathBuf, opt_first_commit: Option<&str>) -> Result<bool, anyhow::Error> {
        if !git_utils::is_git_repo(path) {
            return Ok(false);
        }
        match opt_first_commit {
            None => Ok(true),
            Some(first_commit) => Ok(git_utils::repo_contains_commit(path, first_commit)?),
        }
    }

    pub fn repository_path(&self) -> Option<&str> {
        if !self.path.exists() {
            None
        } else {
            self.path.to_str()
        }
    }

    pub fn open(&mut self) -> &str {
        let repo_path = FileDialog::new().set_location("~/workspace/review-todo").show_open_single_dir().unwrap();

        match repo_path {
            None => "",
            Some(path) => {
                self.path = path;
                self.path.to_str().unwrap()
            }
        }
    }

    pub fn diff_repository(&mut self, start_commit: &str, end_commit: &str) {
        self.current_diff.start_commit = start_commit.to_string();
        self.current_diff.end_commit = end_commit.to_string();

        self.current_diff.file_diff_model.clear();

        let diff_result = git_utils::diff_git_repo(&self.path, &start_commit, &end_commit);
        if let Err(e) = diff_result {
            // TODO proper error handling
            eprintln!("Diff of repo failed: {}", e.to_string());
            return;
        }
        let output_text = diff_result.unwrap();
        output_text.split('\n').filter(|file| false == file.is_empty()).for_each(|file| {
            self.current_diff.file_diff_model.push(ui::DiffFileItem {
                text: file.into(),
                is_reviewed: false,
            })
        });
    }

    pub fn toggle_file_is_reviewed(&mut self, item_index: usize) {
        if let Some(mut item) = self.current_diff.file_diff_model.row_data(item_index) {
            item.is_reviewed = !item.is_reviewed;
            self.current_diff.file_diff_model.set_row_data(item_index, item);
        }
    }

    pub fn diff_file(&self, index: i32) {
        match self.current_diff.file_diff_model.row_data(index as usize) {
            None => eprintln!("Could not found file!"), // TODO proper error handling
            Some(file_item) => {
                if let Err(e) = git_utils::diff_file(&self.path, &self.current_diff.start_commit, &self.current_diff.end_commit, &file_item.text) {
                    // TODO proper error handling
                    eprintln!("File diff failed: {}", e.to_string());
                }
            }
        }
    }

    pub fn file_diff_model(&self) -> Rc<VecModel<ui::DiffFileItem>> {
        self.current_diff.file_diff_model.clone()
    }

    pub fn diff_range(&self) -> (&str, &str) {
        (&self.current_diff.start_commit, &self.current_diff.end_commit)
    }
}
