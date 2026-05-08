#[cfg(windows)]
use std::process::Command;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

#[cfg(not(windows))]
use mockcmd::Command;

use itertools::Itertools;

use which::which;

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
        match change_type.chars().next() {
            Some('A') => ChangeType::Added,
            Some('C') => ChangeType::Copied,
            Some('D') => ChangeType::Deleted,
            Some('M') => ChangeType::Modified,
            Some('R') => ChangeType::Renamed,
            Some('T') => ChangeType::TypChanged,
            Some('U') => ChangeType::Unmerged,
            Some('X') => ChangeType::Unknown,
            Some('B') => ChangeType::Broken,
            _ => ChangeType::Invalid,
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
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(windows)]
macro_rules! git_command {
    ($path:expr, $args:expr) => {
        Command::new("git").current_dir($path).args($args).creation_flags(CREATE_NO_WINDOW)
    };
}

#[cfg(not(windows))]
macro_rules! git_command {
    ($path:expr, $args:expr) => {
        Command::new("git").current_dir($path).args($args)
    };
}

pub fn is_git_repo(path: &Path) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

pub fn _repo_contains_commit(path: &Path, commit: &str) -> anyhow::Result<bool> {
    let args = vec!["cat-file", "-t", commit];
    let output = git_command!(path, args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}

pub fn repo_contains_branch(path: &Path, branch: &str) -> anyhow::Result<bool> {
    let args = vec!["branch", "--list", branch];
    let output = git_command!(path, args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(!msg.is_empty())
}

pub fn diff_git_repo(repo_path: &Path, start_commit: &str, end_commit: &str) -> anyhow::Result<FileDiffMap> {
    let files_change_type = diff_name_status(repo_path, start_commit, end_commit)?;
    let files_stats = query_file_stats(repo_path, start_commit, end_commit, files_change_type)?;
    Ok(files_stats)
}

fn diff_name_status(repo_path: &Path, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, ChangeType>> {
    let mut args = vec!["diff", "--name-status"];

    if !start_commit.is_empty() {
        args.push(start_commit);
    }
    if !end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = git_command!(repo_path, args).output()?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }
    let output_str = std::str::from_utf8(&output.stdout)?;

    output_str
        .lines()
        .map(|line| {
            let infos = line.split_whitespace().collect::<Vec<&str>>();
            if infos.len() < 2 {
                anyhow::bail!("diff_name_status: Malformed status line: {}", line);
            }
            let change_type = ChangeType::from_str(infos[0]);
            let file = if change_type == ChangeType::Renamed {
                if infos.len() < 3 {
                    anyhow::bail!("diff_name_status: Malformed status line in rename status: {}", line);
                }
                infos[2].to_string()
            } else {
                infos[1].to_string()
            };
            Ok((file, change_type))
        })
        .collect()
}

fn query_file_stats(
    repo_path: &Path,
    start_commit: &str,
    end_commit: &str,
    mut files_change_type: HashMap<String, ChangeType>,
) -> anyhow::Result<HashMap<String, DiffStatus>> {
    let mut args = vec!["diff", "-z", "--numstat"];

    if !start_commit.is_empty() {
        args.push(start_commit);
    }
    if !end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = git_command!(repo_path, args).output()?;

    if !output.status.success() {
        return Ok(HashMap::new());
    }

    let output_str = std::str::from_utf8(&output.stdout)?;

    let mut files_stats: HashMap<String, DiffStatus> = HashMap::new();

    let mut iter = output_str.split('\0');

    while let Some(line) = iter.next() {
        if line.is_empty() {
            continue;
        }
        let parts = line.split_whitespace().collect::<Vec<&str>>();
        if parts.len() < 2 {
            continue;
        }

        let added = parts[0].parse::<u32>().unwrap_or(0);
        let removed = parts[1].parse::<u32>().unwrap_or(0);

        let file_path = if parts.len() == 2 {
            iter.nth(1).ok_or_else(|| anyhow::anyhow!("Missing renamed path!"))?
        } else {
            parts[2]
        };

        let change_type = files_change_type.remove(file_path).unwrap_or(ChangeType::Invalid);

        files_stats.insert(
            file_path.to_string(),
            DiffStatus {
                added_lines: added,
                removed_lines: removed,
                change_type,
            },
        );
    }
    Ok(files_stats)
}

pub fn diff_file(repo_path: &Path, start_commit: &str, end_commit: &str, file: &str, diff_tool: &str) -> anyhow::Result<()> {
    let mut args = vec!["difftool", "-U100000", "--no-prompt"];

    let diff_tool = format!("--tool={}", diff_tool);

    args.push(&diff_tool);

    if !start_commit.is_empty() {
        args.push(start_commit);
    }
    if !end_commit.is_empty() {
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

pub fn first_commit(repo_path: &Path) -> anyhow::Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "--reverse", "HEAD"];
    let output = git_command!(repo_path, args).output()?;

    if !output.status.success() {
        anyhow::bail!("first_commit: git command failed!");
    }

    let output_str = std::str::from_utf8(&output.stdout)?;
    let commit = output_str.lines().next().unwrap_or("").trim();

    if commit.is_empty() {
        anyhow::bail!("Git could not find any commit in {}!", repo_path.display());
    }

    Ok(commit.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

pub fn query_commits(repo_path: &Path) -> anyhow::Result<Vec<Commit>> {
    let args = vec!["--no-pager", "log", "--first-parent", "--pretty=format:%h¦%an¦%aI¦%s"];
    let output = git_command!(repo_path, args).output()?;

    if !output.status.success() {
        anyhow::bail!("query_commits: git command failed!");
    }

    let output_str = std::str::from_utf8(&output.stdout)?;

    output_str
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts = line.splitn(4, "¦").collect::<Vec<_>>();
            if parts.len() < 4 {
                anyhow::bail!("query_commits: Malformed git output line: {}", line);
            }
            let date_time = DateTime::parse_from_rfc3339(parts[2]).map_err(|e| anyhow::anyhow!("Invalid date {}: {}", parts[2], e))?;
            Ok(Commit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                date: date_time.to_string(),
                message: parts[3].to_string(),
            })
        })
        .collect()
}

fn query_diff_tools_from_config() -> anyhow::Result<HashSet<String>> {
    let args = vec!["config", "get", "--all", "--show-names", "--regexp", r"difftool\..*\.(cmd|path)"];
    let output = git_command!(dirs::home_dir().unwrap_or_default(), args).output()?;

    if !output.status.success() {
        return Ok(HashSet::new());
    }

    let output_str = std::str::from_utf8(&output.stdout)?;

    Ok(output_str
        .lines()
        .filter_map(|line| {
            let (key, _) = line.split_once(char::is_whitespace)?;
            let parts: Vec<&str> = key.split('.').collect();
            if parts.len() >= 3 && parts[0] == "difftool" {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>())
}

fn query_diff_tools_from_path() -> anyhow::Result<HashSet<String>> {
    const DIFF_TOOLS: &[&str] = &[
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

    let diff_tools = DIFF_TOOLS
        .iter()
        .filter(|tool| which(tool).is_ok())
        .map(|tool| tool.to_string())
        .collect::<HashSet<_>>();

    Ok(diff_tools)
}

pub fn query_diff_tools() -> anyhow::Result<Vec<String>> {
    let mut all_tools = query_diff_tools_from_config()?;
    let tools_from_path = query_diff_tools_from_path()?;

    all_tools.extend(tools_from_path);

    Ok(all_tools.into_iter().sorted().collect())
}

pub fn branch_merge_base(repo_path: &Path, base_branch: &str, feature_branch: &str) -> anyhow::Result<String> {
    let args = ["merge-base", base_branch, feature_branch];
    let output = git_command!(repo_path, args).output()?;
    if !output.status.success() {
        anyhow::bail!("Could not find merge base between {} and {}", base_branch, feature_branch);
    }

    let hash = std::str::from_utf8(&output.stdout)?.trim();

    if hash.is_empty() {
        anyhow::bail!("Git returned an empty merge base hash");
    }

    Ok(hash.to_string())
}

pub fn current_branch(repo_path: &Path) -> anyhow::Result<String> {
    let args = ["branch", "--show-current"];
    let output = git_command!(repo_path, args).output()?;

    if !output.status.success() {
        anyhow::bail!("git branch failed");
    }
    let branch = std::str::from_utf8(&output.stdout)?.trim();
    if branch.is_empty() {
        anyhow::bail!("Could not determine current branch: {}", repo_path.display());
    }
    Ok(branch.to_string())
}

#[cfg(test)]
#[cfg(not(windows))]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::Ok;
    use mockcmd::{CommandMockBuilder, mock, was_command_executed};

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
    fn test_first_commit() -> anyhow::Result<()> {
        let ctx = setup();

        let args = ["rev-list", "--max-parents=0", "--reverse", "HEAD"];

        mock("git")
            .current_dir(&ctx.path)
            .with_args(args)
            .with_stdout("9f89049b7f99682c48474d421ac126316adaed15")
            .register();

        let commit = first_commit(&ctx.path)?;

        let expected_cmd = [&["git"], &args[..]].concat();

        assert!(was_command_executed(&expected_cmd, Some(&ctx.path.to_string_lossy())));

        assert_eq!(commit, "9f89049b7f99682c48474d421ac126316adaed15".to_string());

        Ok(())
    }

    #[test]
    fn test_is_git_repo() {
        let ctx = setup();
        assert!(is_git_repo(&ctx.path));
    }

    #[test]
    fn test_repo_contains_commit() -> anyhow::Result<()> {
        let ctx = setup();
        let commit = "9f89049b7f99682c48474d421ac126316adaed15";
        let args = ["cat-file", "-t", commit];

        mock("git").current_dir(&ctx.path).with_args(args).with_stdout("commit").register();

        let expected_cmd = [&["git"], &args[..]].concat();

        let contains_commit = _repo_contains_commit(&ctx.path, commit)?;

        assert!(was_command_executed(&expected_cmd, Some(&ctx.path.to_string_lossy())));

        assert!(contains_commit);

        Ok(())
    }

    #[test]
    fn test_repo_contains_branch() -> anyhow::Result<()> {
        let ctx = setup();
        let branch = "main";
        let args = ["branch", "--list", branch];

        mock("git").current_dir(&ctx.path).with_args(args).with_stdout("main").register();
        let expected_cmd = [&["git"], &args[..]].concat();

        let contains_branch = repo_contains_branch(&ctx.path, branch)?;

        assert!(was_command_executed(&expected_cmd, Some(&ctx.path.to_string_lossy())));

        assert!(contains_branch);

        Ok(())
    }

    fn git_mock(ctx: &TestContext) -> CommandMockBuilder {
        mock("git").current_dir(&ctx.path)
    }

    #[test]
    fn test_diff_git_repo() -> anyhow::Result<()> {
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
            .with_stdout("6       0       rustfmt.toml\0 137       0       src/lib.rs\0 22  94      src/main.rs\0")
            .register();

        let result = diff_git_repo(&ctx.path, start_commit, end_commit)?;

        let expected_git_status_cmd = [&["git"], &git_status_args[..]].concat();
        assert!(was_command_executed(&expected_git_status_cmd, Some(&ctx.path.to_string_lossy())));

        let expected_git_file_status_cmd = [&["git"], &git_file_status_args[..]].concat();
        assert!(was_command_executed(&expected_git_file_status_cmd, Some(&ctx.path.to_string_lossy())));

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
        assert_eq!(result, expected_stats);

        Ok(())
    }

    #[test]
    fn test_query_commits() -> anyhow::Result<()> {
        let ctx = setup();

        let args = ["--no-pager", "log", "--first-parent", "--pretty=format:%h¦%an¦%aI¦%s"];
        let output = "70989e0¦Christian von Wascinski¦2023-10-16T22:34:17+02:00¦feature: add open comments.\n\
                                    dd02a7c¦Christian von Wascinski¦2023-10-15T16:25:02+02:00¦feature: Add saving notes as todo.txt\n\
                                    9f89049¦Christian von Wascinski¦2023-10-14T10:05:19+02:00¦Initial commit\n";

        git_mock(&ctx).with_args(args).with_stdout(output).register();

        let commits = query_commits(&ctx.path)?;

        let expected_git_cmd = [&["git"], &args[..]].concat();
        assert!(was_command_executed(&expected_git_cmd, Some(&ctx.path.to_string_lossy())));

        assert_eq!(commits.len(), 3);
        let first_commit = commits.last().unwrap();

        assert_eq!(first_commit.hash, "9f89049");
        assert_eq!(first_commit.message, "Initial commit");
        assert_eq!(first_commit.author, "Christian von Wascinski");
        assert_eq!(first_commit.date, "2023-10-14 10:05:19 +02:00");

        Ok(())
    }

    #[test]
    fn test_query_diff_tools() -> anyhow::Result<()> {
        let args = ["config", "get", "--all", "--show-names", "--regexp", "difftool\\..*\\.(cmd|path)"];

        let path = dirs::home_dir().expect("Should determine home directory!");

        mock("git")
            .current_dir(&path)
            .with_args(args)
            .with_stdout("difftool.vscode.cmd code --new-window --wait --diff $LOCAL $REMOTE")
            .register();

        let diff_tools = query_diff_tools()?;

        let expected_git_cmd = [&["git"], &args[..]].concat();
        assert!(was_command_executed(&expected_git_cmd, Some(path.to_str().unwrap_or_default())));

        assert!(!diff_tools.is_empty());

        assert!(diff_tools.contains(&"vscode".to_string()));

        Ok(())
    }

    #[test]
    fn test_current_branch() -> anyhow::Result<()> {
        let ctx = setup();
        let args = ["branch", "--show-current"];

        git_mock(&ctx).with_args(args).with_stdout("main").register();

        let current_branch = current_branch(&ctx.path)?;

        let expected_git_cmd = [&["git"], &args[..]].concat();
        assert!(was_command_executed(&expected_git_cmd, Some(&ctx.path.to_string_lossy())));

        // assert!(current_branch_result.is_ok());
        assert_eq!(&current_branch, "main");

        Ok(())
    }

    #[test]
    fn test_branch_merge_base() -> anyhow::Result<()> {
        let ctx = setup();

        let base = "main";
        let feature = "feature";
        let args = ["merge-base", base, feature];
        let expected_commit = "0x2ac";

        git_mock(&ctx).with_args(args).with_stdout(expected_commit).register();

        let commit = branch_merge_base(&ctx.path, base, feature)?;

        assert_eq!(commit, expected_commit);

        Ok(())
    }
}
