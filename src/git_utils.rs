#[cfg(windows)]
use std::process::Command;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[cfg(not(windows))]
use mockcmd::Command;

use itertools::Itertools;

use which::which;

extern crate dirs;

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
        match change_type.chars().nth(0).expect("Could not get first char!") {
            'A' => ChangeType::Added,
            'C' => ChangeType::Copied,
            'D' => ChangeType::Deleted,
            'M' => ChangeType::Modified,
            'R' => ChangeType::Renamed,
            'T' => ChangeType::TypChanged,
            'U' => ChangeType::Unmerged,
            'X' => ChangeType::Unknown,
            'B' => ChangeType::Broken,
            _default => ChangeType::Invalid,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct DiffStatus {
    pub added_lines: u32,
    pub removed_lines: u32,
    pub change_type: ChangeType,
}

pub type FileDiffMap = HashMap<String, DiffStatus>;

use chrono::DateTime;
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

pub fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<FileDiffMap> {
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
        assert!(infos.len() > 1);

        let change_type = ChangeType::from_str(infos[0]);
        let file = if change_type == ChangeType::Renamed {
            assert_eq!(infos.len(), 3);
            infos[2].to_string()
        } else {
            infos[1].to_string()
        };
        files_change_type.insert(file, change_type);
    }

    Ok(files_change_type)
}

fn query_file_stats(
    repo_path: &PathBuf,
    start_commit: &str,
    end_commit: &str,
    mut files_change_type: HashMap<String, ChangeType>,
) -> anyhow::Result<HashMap<String, DiffStatus>> {
    let mut args = vec!["diff", "-z", "--numstat"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = git_command!(repo_path, args).output()?;
    let string_output = String::from_utf8(output.stdout.trim_ascii().to_vec())?;

    let mut files_stats: HashMap<String, DiffStatus> = HashMap::new();
    let parse_line_number = |number_str: &str| -> anyhow::Result<u32> {
        if number_str.contains("-") {
            Ok(0)
        } else {
            number_str.parse::<u32>().map_err(|e| anyhow::format_err!(e.to_string()))
        }
    };

    let lines = string_output.split("\0").collect::<Vec<&str>>();
    let mut iter = lines.iter();

    while let Some(line) = iter.next() {
        if line.is_empty() {
            continue;
        }
        let infos = line.split_whitespace().collect::<Vec<&str>>();
        if infos.len() == 3 {
            let file = infos[2].to_string();
            let change_type = if let Some(ct) = files_change_type.remove(&file) {
                ct
            } else {
                ChangeType::Invalid
            };
            files_stats.insert(
                file,
                DiffStatus {
                    added_lines: parse_line_number(infos[0])?,
                    removed_lines: parse_line_number(infos[1])?,
                    change_type,
                },
            );
        } else {
            let added_lines = parse_line_number(infos[0])?;
            let removed_lines = parse_line_number(infos[1])?;
            let _old_file = iter.next(); // TODO display it as additional information

            let new_file = iter.next().expect("Renamed new file name missing");
            let change_type = if let Some(ct) = files_change_type.remove(*new_file) {
                ct
            } else {
                ChangeType::Invalid
            };
            files_stats.insert(
                new_file.to_string(),
                DiffStatus {
                    added_lines,
                    removed_lines,
                    change_type,
                },
            );
        }
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

    {
        use std::process::Command;
        git_command!(repo_path, args).spawn()?;
    }

    Ok(())
}

pub fn first_commit(repo_path: &PathBuf) -> anyhow::Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = git_command!(repo_path, args).output()?;

    String::from_utf8(output.stdout.trim_ascii().to_vec()).map_err(|e| anyhow::Error::from(e))
}

pub struct Commit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

pub fn query_commits(repo_path: &PathBuf) -> anyhow::Result<Vec<Commit>> {
    let mut commits = Vec::<Commit>::new();
    let args = vec!["--no-pager", "log", "--first-parent", "--pretty=format:\"%h¦%an¦%aI¦%s\""];
    let output = git_command!(repo_path, args).output()?;
    let output_string = String::from_utf8(output.stdout.trim_ascii().to_vec())?;

    for line in output_string.split("\n") {
        let line = line.trim_matches('"');
        let mut iter = line.splitn(4, "¦");

        let hash = iter.next().expect("Could get read sha!").to_string();
        let author = iter.next().expect("Could get read author!").to_string();
        let date = iter.next().expect("Could get read date!").to_string();
        let message = iter.next().expect("Could get read message!").to_string();

        let date_time = DateTime::parse_from_rfc3339(&date).expect("Could parse date!");

        let commit = Commit {
            hash,
            author,
            date: date_time.to_string(),
            message,
        };
        commits.push(commit);
    }
    Ok(commits)
}

fn query_diff_tools_from_config() -> anyhow::Result<HashSet<String>> {
    let args = vec!["config", "get", "--all", "--show-names", "--regexp", "difftool\\..*\\.(cmd|path)"];
    let output = git_command!(dirs::home_dir().unwrap_or_default(), args).output()?;
    let output_string = String::from_utf8(output.stdout.trim_ascii().to_vec())?;

    let mut diff_tools = HashSet::new();

    for line in output_string.split("\n") {
        if let Some((diff_desc, _)) = line.split_once(char::is_whitespace) {
            if let Some((_, diff_tool, _)) = diff_desc.split(".").into_iter().collect_tuple() {
                diff_tools.insert(diff_tool.to_string());
            }
        }
    }

    Ok(diff_tools)
}

fn query_diff_tools_from_path() -> anyhow::Result<HashSet<String>> {
    let tools = [
        "araxis",
        "kdiff3",
        "meld",
        "smerge",
        "bc",
        "bc3",
        "bc4",
        "codecompare",
        "deltawalker",
        "diffmerge",
        "diffuse",
        "ecmerge",
        "emerge",
        "examdiff",
        "guiffy",
        "gvimdiff",
        "kompare",
        "nvimdiff",
        "opendiff",
        "p4merge",
        "tkdiff",
        "vimdiff",
        "winmerge",
        "xxdiff",
    ];
    let mut diff_tools = HashSet::new();
    for tool in tools {
        if let Ok(_) = which(tool) {
            diff_tools.insert(tool.to_string());
        }
    }
    Ok(diff_tools)
}

pub fn query_diff_tools() -> anyhow::Result<Vec<String>> {
    let mut tools_from_config = query_diff_tools_from_config()?;
    let tools_from_path = query_diff_tools_from_path()?;
    tools_from_config.extend(tools_from_path);
    Ok(tools_from_config.iter().cloned().sorted().collect_vec())
}

#[cfg(test)]
#[cfg(not(windows))]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use mockcmd::{mock, was_command_executed, CommandMockBuilder};

    use crate::git_utils::*;

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

        let args = ["rev-list", "--max-parents=0", "HEAD"];

        mock("git")
            .current_dir(&ctx.path)
            .with_args(&args)
            .with_stdout("9f89049b7f99682c48474d421ac126316adaed15")
            .register();

        let result = first_commit(&ctx.path);

        let expected_cmd = [&["git"], &args[..]].concat();

        assert!(was_command_executed(&expected_cmd, Some(ctx.path.to_str().unwrap_or_default())));

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
        let commit = "9f89049b7f99682c48474d421ac126316adaed15";
        let args = ["cat-file", "-t", commit];

        mock("git").current_dir(&ctx.path).with_args(&args).with_stdout("commit").register();

        let expected_cmd = [&["git"], &args[..]].concat();

        let result = repo_contains_commit(&ctx.path, commit);

        assert!(was_command_executed(&expected_cmd, Some(ctx.path.to_str().unwrap_or_default())));

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    fn git_mock(ctx: &TestContext) -> CommandMockBuilder {
        mock("git").current_dir(&ctx.path)
    }

    #[test]
    fn test_diff_git_repo() {
        let ctx = setup();

        let start_commit = "70989e0fbda7919d357c0183e62294423f3d9425";
        let end_commit = "68c5f4631d6e6b040d7887f7445cf1ad4006e1a5";
        let git_status_args = ["diff", "--name-status", start_commit, end_commit];

        git_mock(&ctx)
            .with_args(git_status_args)
            .with_stdout("A       rustfmt.toml\nA       src/lib.rs\nM       src/main.rs\n")
            .register();

        let git_file_status_args = ["diff", "-z", "--numstat", start_commit, end_commit];

        git_mock(&ctx)
            .with_args(git_file_status_args)
            .with_stdout("6       0       rustfmt.toml\0137       0       src/lib.rs\022  94      src/main.rs\0")
            .register();

        let result = diff_git_repo(&ctx.path, start_commit, end_commit);

        let expected_git_status_cmd = [&["git"], &git_status_args[..]].concat();
        assert!(was_command_executed(&expected_git_status_cmd, Some(ctx.path.to_str().unwrap_or_default())));

        let expected_git_file_status_cmd = [&["git"], &git_file_status_args[..]].concat();
        assert!(was_command_executed(&expected_git_file_status_cmd, Some(ctx.path.to_str().unwrap_or_default())));

        assert!(result.is_ok());
        let expected_stats = HashMap::from([
            (
                "src/lib.rs".to_string(),
                DiffStatus {
                    added_lines: 137,
                    removed_lines: 0,
                    change_type: ChangeType::Added,
                },
            ),
            (
                "src/main.rs".to_string(),
                DiffStatus {
                    added_lines: 22,
                    removed_lines: 94,
                    change_type: ChangeType::Modified,
                },
            ),
            (
                "rustfmt.toml".to_string(),
                DiffStatus {
                    added_lines: 6,
                    removed_lines: 0,
                    change_type: ChangeType::Added,
                },
            ),
        ]);
        assert_eq!(result.unwrap(), expected_stats);
    }

    #[test]
    fn test_query_commits() {
        let ctx = setup();

        let args = ["--no-pager", "log", "--first-parent", "--pretty=format:\"%h¦%an¦%aI¦%s\""];
        let output = "70989e0¦Christian von Wascinski¦2023-10-16T22:34:17+02:00¦feature: add open comments.\n\
                                    dd02a7c¦Christian von Wascinski¦2023-10-15T16:25:02+02:00¦feature: Add saving notes as todo.txt\n\
                                    9f89049¦Christian von Wascinski¦2023-10-14T10:05:19+02:00¦Initial commit\n";

        git_mock(&ctx).with_args(args).with_stdout(output).register();

        let commits = query_commits(&ctx.path);

        let expected_git_cmd = [&["git"], &args[..]].concat();
        assert!(was_command_executed(&expected_git_cmd, Some(ctx.path.to_str().unwrap_or_default())));

        assert!(commits.is_ok());
        let commits = commits.unwrap();

        assert_eq!(commits.len(), 3);
        let first_commit = commits.last().unwrap();

        assert_eq!(first_commit.hash, "9f89049");
        assert_eq!(first_commit.message, "Initial commit");
        assert_eq!(first_commit.author, "Christian von Wascinski");
        assert_eq!(first_commit.date, "2023-10-14 10:05:19 +02:00");
    }

    #[test]
    fn test_query_diff_tools() {
        let args = ["config", "get", "--all", "--show-names", "--regexp", "difftool\\..*\\.(cmd|path)"];

        let path = dirs::home_dir().unwrap_or_default();

        mock("git")
            .current_dir(&path)
            .with_args(args)
            .with_stdout("difftool.vscode.cmd code --new-window --wait --diff $LOCAL $REMOTE")
            .register();

        let diff_tools_result = query_diff_tools();

        let expected_git_cmd = [&["git"], &args[..]].concat();
        assert!(was_command_executed(&expected_git_cmd, Some(path.to_str().unwrap_or_default())));

        assert!(diff_tools_result.is_ok());

        let diff_tools = diff_tools_result.unwrap();
        assert!(diff_tools.len() > 0);

        assert!(diff_tools.contains(&"vscode".to_string()));
    }
}
