use std::{collections::HashMap, path::PathBuf, process::Command};

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Clone)]
pub enum ChangeType {
    Invalid,
    Added,
    Copied,
    Deleted,
    Modified,
    Renamed,
    TypChanged,
    Unmerged,
    Unknown,
    Broken,
}

impl ChangeType {
    pub fn from_str(change_type: &str) -> ChangeType {
        match change_type {
            "A" => ChangeType::Added,
            "C" => ChangeType::Copied,
            "D" => ChangeType::Deleted,
            "M" => ChangeType::Modified,
            "R" => ChangeType::Renamed,
            "T" => ChangeType::TypChanged,
            "U" => ChangeType::Unmerged,
            "X" => ChangeType::Unknown,
            "B" => ChangeType::Broken,
            _default => ChangeType::Invalid,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FileStat {
    pub added_lines: u32,
    pub removed_lines: u32,
    pub change_type: ChangeType,
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
macro_rules! git_command {
    ($path:expr, $args:expr) => {
        // NOTE create no window
        Command::new("git").current_dir($path).args($args).creation_flags(0x08000000)
    };
}

#[cfg(not(windows))]
macro_rules! git_command {
    ($path:expr, $args:expr) => {
        Command::new("git").current_dir($path).args($args)
    };
}

pub fn is_git_repo(path: &PathBuf) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

pub fn repo_contains_commit(path: &PathBuf, commit: &str) -> anyhow::Result<bool> {
    let args = vec!["cat-file", "-t", commit];
    let output = git_command!(path, args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}

pub fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, FileStat>> {
    let files_change_type = diff_name_status(repo_path, start_commit, end_commit)?;
    let files_stats = query_file_stats(repo_path, start_commit, end_commit, files_change_type)?;
    Ok(files_stats)
}

fn diff_name_status(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, ChangeType>> {
    let mut args = vec!["diff", "--name-status"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = git_command!(repo_path, args).output().expect("git diff name-status not working!");

    let string_output = String::from_utf8(output.stdout.trim_ascii().to_vec()).expect("String conversion invalid!");
    let mut files_change_type: HashMap<String, ChangeType> = HashMap::new();

    for line in string_output.lines().collect::<Vec<&str>>() {
        let infos = line.split_whitespace().collect::<Vec<&str>>();
        assert_eq!(infos.len(), 2);

        let file = infos[1].to_string();
        files_change_type.insert(file, ChangeType::from_str(infos[0]));
    }

    Ok(files_change_type)
}

fn query_file_stats(
    repo_path: &PathBuf,
    start_commit: &str,
    end_commit: &str,
    mut files_change_type: HashMap<String, ChangeType>,
) -> anyhow::Result<HashMap<String, FileStat>> {
    let mut args = vec!["diff", "--numstat"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = git_command!(repo_path, args).output()?;
    let string_output = String::from_utf8(output.stdout.trim_ascii().to_vec())?;

    let mut files_stats: HashMap<String, FileStat> = HashMap::new();
    let parse_line_number = |number_str: &str| -> anyhow::Result<u32> {
        if number_str.contains("-") {
            Ok(0)
        } else {
            number_str.parse::<u32>().map_err(|e| anyhow::format_err!(e.to_string()))
        }
    };

    for line in string_output.lines().collect::<Vec<&str>>() {
        if line.is_empty() {
            continue;
        }
        let infos = line.split_whitespace().collect::<Vec<&str>>();

        assert_eq!(infos.len(), 3);

        let key = infos[2].to_string();
        let change_type = if let Some(ct) = files_change_type.remove(&key) {
            ct
        } else {
            ChangeType::Invalid
        };
        let value = FileStat {
            added_lines: parse_line_number(infos[0])?,
            removed_lines: parse_line_number(infos[1])?,
            change_type: change_type,
        };
        files_stats.insert(key, value);
    }
    Ok(files_stats)
}

pub fn diff_file(repo_path: &PathBuf, start_commit: &str, end_commit: &str, file: &str, diff_tool: &str) -> anyhow::Result<()> {
    let mut args = vec!["difftool", "-U100000", "--no-prompt"];

    let diff_tool = format!("--tool={}", diff_tool);

    args.push(&diff_tool);

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    args.push("--");
    args.push(file);

    git_command!(repo_path, args).spawn()?;
    Ok(())
}

pub fn first_commit(repo_path: &PathBuf) -> anyhow::Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = git_command!(repo_path, args).output()?;

    String::from_utf8(output.stdout.trim_ascii().to_vec()).map_err(|e| anyhow::Error::from(e))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use crate::git_utils::{diff_git_repo, first_commit, is_git_repo, repo_contains_commit, ChangeType, FileStat};

    struct TestContext {
        path: PathBuf,
    }

    fn setup() -> TestContext {
        let manifest_dir = std::env!("CARGO_MANIFEST_DIR");
        TestContext {
            path: PathBuf::from(manifest_dir),
        }
    }

    #[test]
    fn test_first_commit() {
        let ctx = setup();
        let result = first_commit(&ctx.path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "9f89049b7f99682c48474d421ac126316adaed15".to_string());
    }
    #[test]
    fn test_is_git_repo() {
        let ctx = setup();
        assert!(is_git_repo(&ctx.path));
    }

    #[test]
    fn test_repo_contains_commit() {
        let ctx = setup();
        let result = repo_contains_commit(&ctx.path, "9f89049b7f99682c48474d421ac126316adaed15");
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
    #[test]
    fn test_diff_git_repo() {
        let ctx = setup();

        let start_commit = "70989e0fbda7919d357c0183e62294423f3d9425";
        let end_commit = "68c5f4631d6e6b040d7887f7445cf1ad4006e1a5";
        let result = diff_git_repo(&ctx.path, start_commit, end_commit);
        assert!(result.is_ok());
        let expected_stats = HashMap::from([
            (
                "src/lib.rs".to_string(),
                FileStat {
                    added_lines: 137,
                    removed_lines: 0,
                    change_type: ChangeType::Added,
                },
            ),
            (
                "src/main.rs".to_string(),
                FileStat {
                    added_lines: 22,
                    removed_lines: 94,
                    change_type: ChangeType::Modified,
                },
            ),
            (
                "rustfmt.toml".to_string(),
                FileStat {
                    added_lines: 6,
                    removed_lines: 0,
                    change_type: ChangeType::Added,
                },
            ),
        ]);
        assert_eq!(result.unwrap(), expected_stats);
    }
}
