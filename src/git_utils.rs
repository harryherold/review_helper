use anyhow::Result;
use std::{path::PathBuf, process::Command};

pub fn is_git_repo(path: &PathBuf) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

pub fn repo_contains_commit(path: &PathBuf, commit: &str) -> Result<bool, anyhow::Error> {
    let args = vec!["cat-file", "-t", commit];
    let output = Command::new("git").current_dir(path).args(args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}

pub fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> Result<String> {
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

pub fn diff_file(repo_path: &PathBuf, start_commit: &str, end_commit: &str, file: &str) -> Result<()> {
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

pub fn first_commit(repo_path: &PathBuf) -> Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = Command::new("git").current_dir(repo_path).args(args).output()?;

    String::from_utf8(output.stdout.trim_ascii().to_vec()).map_err(|e| anyhow::Error::from(e))
}
