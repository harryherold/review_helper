use std::{path::PathBuf, process::Command, rc::Rc};

use native_dialog::FileDialog;
use slint::{Model, VecModel};

use anyhow::{Ok, Result};

use crate::{config::Config, ui};

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
    pub fn new(config: &Config) -> Repository {
        let mut repo = Repository {
            path: PathBuf::from(config.repo_path.to_string()),
            current_diff: Diff::new(),
        };
        repo.diff_repository(&config.start_diff, &config.end_diff);
        repo
    }

    pub fn is_repo_valid(path: &PathBuf, opt_first_commit: Option<&str>) -> Result<bool, anyhow::Error> {
        if !is_git_repo(path) {
            return Ok(false);
        }
        match opt_first_commit {
            None => Ok(true),
            Some(first_commit) => Ok(repo_contains_commit(path, first_commit)?),
        }
    }

    pub fn repository_path(&self) -> &str {
        self.path.to_str().expect("Repo path not set")
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

fn is_git_repo(path: &PathBuf) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

fn repo_contains_commit(path: &PathBuf, commit: &str) -> Result<bool, anyhow::Error> {
    let args = vec!["cat-file", "-t", commit];
    let output = Command::new("git").current_dir(path).args(args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}
