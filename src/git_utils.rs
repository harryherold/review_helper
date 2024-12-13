use std::{collections::HashMap, path::PathBuf, process::Command};

#[derive(Debug)]
pub struct FileStat {
    pub added_lines: u32,
    pub removed_lines: u32,
}

pub fn is_git_repo(path: &PathBuf) -> bool {
    let git_folder = path.join(PathBuf::from(".git"));
    git_folder.is_dir()
}

pub fn repo_contains_commit(path: &PathBuf, commit: &str) -> anyhow::Result<bool> {
    let args = vec!["cat-file", "-t", commit];
    let output = Command::new("git").current_dir(path).args(args).output()?;
    let msg = String::from_utf8(output.stdout)?;
    Ok(msg.contains("commit"))
}

pub fn diff_git_repo(repo_path: &PathBuf, start_commit: &str, end_commit: &str) -> anyhow::Result<HashMap<String, FileStat>> {
    let mut args = vec!["diff", "--numstat"];

    if false == start_commit.is_empty() {
        args.push(start_commit);
    }
    if false == end_commit.is_empty() {
        args.push(end_commit);
    }

    let output = Command::new("git").current_dir(repo_path).args(args).output()?;
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
        let value = FileStat {
            added_lines: parse_line_number(infos[0])?,
            removed_lines: parse_line_number(infos[1])?,
        };
        files_stats.insert(key, value);
    }
    Ok(files_stats)
}

pub fn diff_file(repo_path: &PathBuf, start_commit: &str, end_commit: &str, file: &str) -> anyhow::Result<()> {
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

pub fn first_commit(repo_path: &PathBuf) -> anyhow::Result<String> {
    let args = vec!["rev-list", "--max-parents=0", "HEAD"];
    let output = Command::new("git").current_dir(repo_path).args(args).output()?;

    String::from_utf8(output.stdout.trim_ascii().to_vec()).map_err(|e| anyhow::Error::from(e))
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, process::Command};

    #[derive(Debug)]
    struct FileStat {
        added_lines: u32,
        removed_lines: u32,
    }
    enum ChangeType {
        New,
        Gone,
        Modify,
    }
    impl ChangeType {
        fn as_str(&self) -> &str {
            match *self {
                ChangeType::New => "A",
                ChangeType::Gone => "D",
                ChangeType::Modify => "M",
            }
        }
    }

    #[test]
    fn test_query_diff_stats() {
        let repo_path = PathBuf::from("/home/harry/workspace/review-todo");
        let start_commit = "c13426875795b97d79c03c1dbf56dc2c87164b34";
        let end_commit = "98bad3587d5810b300959bdb6c79b811c8b1c2cd";
        let args = vec!["diff", start_commit, end_commit, "--numstat"];
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .expect("git diff numstats not working!");
        let string_output = String::from_utf8(output.stdout.trim_ascii().to_vec()).expect("String conversion invalid!");
        let mut files_stats: HashMap<String, FileStat> = HashMap::new();

        let parse_line_number = |number_str: &str| -> anyhow::Result<u32> {
            if number_str.contains("-") {
                Ok(0)
            } else {
                number_str.parse::<u32>().map_err(|e| anyhow::format_err!(e.to_string()))
            }
        };

        for line in string_output.lines().collect::<Vec<&str>>() {
            let infos = line.split_whitespace().collect::<Vec<&str>>();
            assert_eq!(infos.len(), 3);

            let key = infos[2].to_string();
            let value = FileStat {
                added_lines: parse_line_number(infos[0]).expect("parse error"),
                removed_lines: parse_line_number(infos[1]).expect("parse error"),
            };
            println!("{} => {:?}", &key, &value);
            files_stats.insert(key, value);
        }
        // println!("lines {}", lines.len());
    }
}
