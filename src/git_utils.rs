use std::{collections::HashMap, os::windows::process::CommandExt, path::PathBuf, process::Command};

const CREATE_NO_WINDOW: u32 = 0x08000000;

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

#[derive(Debug)]
pub struct FileStat {
    pub added_lines: u32,
    pub removed_lines: u32,
    pub change_type: ChangeType,
}

pub fn is_git_repo(path: &PathBuf) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

pub fn repo_contains_commit(path: &PathBuf, commit: &str) -> anyhow::Result<bool> {
    let args = vec!["cat-file", "-t", commit];
    let output = Command::new("git").current_dir(path).args(args).creation_flags(CREATE_NO_WINDOW).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}

pub fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, FileStat>> {
    let files_change_type = add_change_type(repo_path, start_commit, end_commit)?;
    let files_stats = query_file_stats(repo_path, start_commit, end_commit, files_change_type)?;
    Ok(files_stats)
}

fn add_change_type(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, ChangeType>> {
    let mut args = vec!["diff", "--name-status"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .expect("git diff name-status not working!");

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

    let output = Command::new("git").current_dir(repo_path).args(args).creation_flags(CREATE_NO_WINDOW).output()?;
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

    Command::new("git").current_dir(repo_path).args(args).creation_flags(CREATE_NO_WINDOW).spawn()?;
    Ok(())
}

pub fn first_commit(repo_path: &PathBuf) -> anyhow::Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = Command::new("git").current_dir(repo_path).args(args).creation_flags(CREATE_NO_WINDOW).output()?;

    String::from_utf8(output.stdout.trim_ascii().to_vec()).map_err(|e| anyhow::Error::from(e))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, default, path::PathBuf, process::Command};

    #[derive(Debug)]
    struct FileStat {
        added_lines: u32,
        removed_lines: u32,
    }
    #[derive(Debug)]
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
        // fn as_str(&self) -> &str {
        //     match *self {
        //         ChangeType::Added => "A",
        //         ChangeType::Copied => "C",
        //         ChangeType::Deleted => "D",
        //         ChangeType::Modified => "M",
        //         ChangeType::Renamed => "R",
        //         ChangeType::TypChanged => "T",
        //         ChangeType::Unmerged => "U",
        //         ChangeType::Unknown => "X",
        //         ChangeType::Broken => "B",
        //     }
        // }
    }

    #[test]
    fn test_query_diff_stats() {
        let repo_path = PathBuf::from("/home/harry/workspace/review-todo");
        let start_commit = "c13426875795b97d79c03c1dbf56dc2c87164b34";
        let end_commit = "98bad3587d5810b300959bdb6c79b811c8b1c2cd";
        let args = vec!["diff", start_commit, end_commit, "--name-status"];
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .expect("git diff numstats not working!");
        let string_output = String::from_utf8(output.stdout.trim_ascii().to_vec()).expect("String conversion invalid!");

        for line in string_output.lines().collect::<Vec<&str>>() {
            let infos = line.split_whitespace().collect::<Vec<&str>>();
            assert_eq!(infos.len(), 2);

            let file = infos[1].to_string();
            let change_type = ChangeType::from_str(infos[0]);
            println!("{} => {:?}", file, change_type);
        }
        // println!("lines {}", lines.len());
    }
}
