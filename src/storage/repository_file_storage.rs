use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use toml::{Table, Value};

use crate::storage::repository_storage::{DiffRangeStore, FileDiffStore, NoteStore, ReviewName, ReviewStore};
use crate::storage::{RepositoryName, RepositoryStore, ReviewHelperStorage};

const NOTE_FILE_NAME: &str = "notes.md";

#[derive(Debug, Default, Clone)]
pub struct ReviewHelperFileStorage {
    storage_path: PathBuf,
}

impl ReviewHelperFileStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { storage_path: path }
    }
}

fn is_toml(path: &PathBuf) -> bool {
    match path.extension() {
        Some(e) => e == "toml",
        None => false,
    }
}

impl ReviewHelperStorage for ReviewHelperFileStorage {
    fn load_repositories(&self) -> anyhow::Result<Vec<RepositoryStore>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }

        let nested_directories = fs::read_dir(&self.storage_path)?
            .filter(|r| match r {
                Ok(dir_entry) => dir_entry.path().is_dir(),
                Err(_) => false,
            })
            .map(|r| r.expect("Errors should be filtered!").path())
            .collect::<Vec<PathBuf>>();

        // TODO refactor
        let mut tomls = Vec::new();

        for directory in &nested_directories {
            let result = fs::read_dir(directory)?.find(|entry| match entry {
                Ok(dir_entry) => dir_entry.path().is_file() && is_toml(&dir_entry.path()),
                Err(_) => false,
            });
            if let Some(Ok(repo_toml)) = result {
                tomls.push(repo_toml.path());
            }
        }

        let mut repositories = Vec::new();

        for toml in tomls {
            let contents = fs::read_to_string(&toml)?;
            let table = contents.parse::<Table>()?;
            let mut repository_store = RepositoryStore::default();
            if let Some(path) = table["path"].as_str() {
                repository_store.path = PathBuf::from(path);
            }
            if let Some(first_commit) = table["first_commit"].as_str() {
                repository_store.first_commit = first_commit.to_string();
            }
            if let Some(name) = table["name"].as_str() {
                repository_store.name = name.into();
            }
            if let Some(base_branch) = table["base_branch"].as_str() {
                repository_store.base_branch = base_branch.to_string();
            }
            repositories.push(repository_store);
        }

        Ok(repositories)
    }

    fn save_repository(&self, repository_store: &RepositoryStore) -> anyhow::Result<()> {
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)?;
        }
        let mut repository_sub_dir = self.storage_path.clone();
        repository_sub_dir.push(repository_store.name.as_str());

        if !repository_sub_dir.exists() {
            fs::create_dir(&repository_sub_dir)?;
        }

        repository_sub_dir.push(repository_store.name.as_str());
        repository_sub_dir.set_extension("toml");

        let mut table = Table::new();
        table.insert("path".to_string(), Value::String(repository_store.path.to_str().unwrap_or_default().into()));
        table.insert("first_commit".to_string(), Value::String(repository_store.first_commit.clone()));
        table.insert("name".to_string(), Value::String(String::from(repository_store.name.as_str())));
        table.insert("base_branch".to_string(), Value::String(String::from(repository_store.base_branch.as_str())));

        let mut file = File::create(&repository_sub_dir)?;

        let contents = toml::to_string_pretty(&table)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    fn load_review_names(&self, repository_name: &RepositoryName) -> anyhow::Result<Vec<ReviewName>> {
        let mut repository_path = self.storage_path.clone();
        repository_path.push(PathBuf::from(String::from(repository_name)));
        if !repository_path.exists() {
            return Err(anyhow::format_err!("Repository directory does not exists!"));
        }

        let has_toml = |path: &PathBuf| -> bool {
            match fs::read_dir(&path) {
                Err(_) => false,
                Ok(mut read_dir) => read_dir.any(|r| match r {
                    Err(_) => false,
                    Ok(dir_entry) => {
                        let p = dir_entry.path();
                        let ext = p.extension().unwrap_or_default();
                        ext == "toml"
                    }
                }),
            }
        };

        let review_directories = fs::read_dir(&repository_path)?
            .filter(|r| match r {
                Ok(dir_entry) => dir_entry.path().is_dir() && has_toml(&dir_entry.path()),
                Err(_) => false,
            })
            .map(|r| {
                let file_name_result = r.expect("Errors should be filtered!").file_name();
                match file_name_result.to_str() {
                    Some(file_name) => ReviewName::from(file_name),
                    None => panic!("Could not convert filename to &str"),
                }
            })
            .collect::<Vec<ReviewName>>();

        Ok(review_directories)
    }
    fn load_review(&self, repository_name: &RepositoryName, review_name: &ReviewName) -> anyhow::Result<Option<ReviewStore>> {
        let file_name = PathBuf::from(format!("{}.toml", review_name.as_str()));
        let review_dir_path = self.storage_path.join(repository_name.as_str()).join(review_name.as_str());
        let review_file_path = review_dir_path.clone().join(file_name);
        if !review_file_path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(review_file_path)?;
        let table = contents.parse::<Table>()?;

        let mut diff_range = DiffRangeStore::default();
        if let Some(start) = table["start_diff"].as_str() {
            diff_range.start = start.to_string();
        }
        if let Some(end) = table["end_diff"].as_str() {
            diff_range.end = end.to_string();
        }

        let mut review_store = ReviewStore::default();
        review_store.diff_range = diff_range;

        if table.contains_key("diff_files")
            && let Some(diff_files) = table["diff_files"].as_array()
        {
            for diff_file in diff_files {
                if let Some(diff_file_table) = diff_file.as_table() {
                    let mut file_diff_item = FileDiffStore::default();
                    if let Some(file_name) = diff_file_table["file_name"].as_str() {
                        file_diff_item.file_path = PathBuf::from(file_name);
                    }
                    if let Some(is_reviewed) = diff_file_table["is_reviewed"].as_bool() {
                        file_diff_item.is_reviewed = is_reviewed;
                    }
                    review_store.file_diff_list.push(file_diff_item);
                }
            }
        }
        let note_file = review_dir_path.join(NOTE_FILE_NAME);
        if note_file.exists() {
            review_store.notes = load_notes(note_file)?;
        }

        Ok(Some(review_store))
    }
    fn save_review_notes(&self, repository_name: &RepositoryName, review_name: &ReviewName, notes: &[&NoteStore]) -> anyhow::Result<()> {
        let repository_path = self.storage_path.join(repository_name.as_str());
        if !repository_path.exists() {
            return Err(anyhow::format_err!("Respository does not exist!"));
        }
        let review_dir_path = repository_path.join(review_name.as_str());
        if !review_dir_path.exists() {
            fs::create_dir(&review_dir_path)?;
        }

        let note_file = review_dir_path.join(NOTE_FILE_NAME);
        save_notes(&notes, note_file)
    }
    fn save_review_file_diffs(
        &self,
        repository_name: &RepositoryName,
        review_name: &ReviewName,
        diff_range: &DiffRangeStore,
        file_diffs: &[&FileDiffStore],
    ) -> anyhow::Result<()> {
        let file_name = PathBuf::from(format!("{}.toml", review_name.as_str()));
        let repository_path = self.storage_path.join(repository_name.as_str());
        if !repository_path.exists() {
            return Err(anyhow::format_err!("Respository does not exist!"));
        }
        let review_dir_path = repository_path.join(review_name.as_str());
        if !review_dir_path.exists() {
            fs::create_dir(&review_dir_path)?;
        }
        let review_file_path = review_dir_path.clone().join(file_name);

        let mut table = Table::new();
        table.insert("start_diff".to_string(), Value::String(diff_range.start.clone()));
        table.insert("end_diff".to_string(), Value::String(diff_range.end.clone()));

        let file_diff_list: Vec<Value> = file_diffs
            .iter()
            .map(|file_diff_item| {
                let mut table = Table::new();
                table.insert("file_name".to_string(), Value::String(file_diff_item.file_path.to_string_lossy().to_string()));
                table.insert("is_reviewed".to_string(), Value::Boolean(file_diff_item.is_reviewed));
                Value::Table(table)
            })
            .collect();
        table.insert("diff_files".to_string(), Value::Array(file_diff_list));

        let mut file = File::create(&review_file_path)?;

        let contents = toml::to_string_pretty(&table)?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    }
}

fn load_notes(note_file: PathBuf) -> anyhow::Result<Vec<NoteStore>> {
    let to_note = |line: &str| -> Option<(bool, String)> {
        let pos = line.find("[")?;
        let is_done = false == line.get(pos + 1..)?.starts_with("]");
        let text: String = if is_done {
            let pos = line.find("]")?;
            line.get(pos + 1..)?.trim().to_string()
        } else {
            line.get(pos + 2..)?.trim().to_string()
        };
        Some((is_done, text))
    };
    let to_file = |line: &str| -> Option<String> {
        let start = line.find("'")? + 1;
        let end = line.rfind("'")?;
        Some(line.get(start..end)?.to_string())
    };
    let buffer = fs::read_to_string(note_file)?;
    let mut notes = Vec::new();
    let mut iter = buffer.lines().into_iter();
    let mut context = String::new();

    while let Some(line) = iter.next() {
        let line = line.trim();
        if line.starts_with("#") {
            context = to_file(line).expect("Error while parsing heading");
        } else if line.starts_with("*") {
            let (is_done, text) = to_note(line).expect("Error while parsing ListItem");
            notes.push(NoteStore {
                text,
                context: context.clone(),
                is_done,
            });
        }
    }
    anyhow::Ok(notes)
}

fn save_notes(notes: &[&NoteStore], note_file: PathBuf) -> anyhow::Result<()> {
    let mut general_notes = Vec::<String>::new();
    let mut file_notes = BTreeMap::<String, Vec<String>>::new();

    let note_item_to_string = |item: &NoteStore| -> String { format!("* [{}] {}", if item.is_done { "x" } else { "" }, item.text) };

    for item in notes {
        let notes: &mut Vec<String> = if item.context.is_empty() {
            &mut general_notes
        } else {
            file_notes.entry(item.context.to_string()).or_insert(Vec::new())
        };
        notes.push(note_item_to_string(item));
    }
    let mut file = OpenOptions::new().create(true).truncate(true).write(true).open(note_file)?;

    for note in general_notes {
        write!(file, "{}\n", note)?;
    }

    write!(file, "\n")?;

    for (file_name, notes) in file_notes {
        write!(file, "# Notes of '{}'\n", file_name)?;
        for note in notes {
            write!(file, "{}\n", note)?;
        }
        write!(file, "\n")?;
    }

    anyhow::Ok(())
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::storage::repository_storage::{DiffRangeStore, FileDiffStore, RepositoryStore, ReviewName};

    use super::*;
    use std::{
        collections::HashSet,
        env,
        fs::{self, File},
    };

    fn create_repo(mut path: PathBuf, name: &str, contents: &str) {
        path.push(name);

        if !path.exists() {
            assert!(fs::create_dir_all(&path).is_ok());
        }
        path.push(name);
        path.set_extension("toml");
        File::create(&path).expect("Could not create repo!");
        fs::write(&path, contents).expect("Write to repo toml failed!");
    }

    fn create_review(mut path: PathBuf, repository_name: &str, review_name: &str, contents: &str, notes: Vec<NoteStore>) {
        path.push(repository_name);
        path.push(review_name);
        if !path.exists() {
            assert!(fs::create_dir_all(&path).is_ok());
        }
        if !notes.is_empty() {
            let note_file = path.join(NOTE_FILE_NAME);
            assert!(save_notes(&notes.iter().collect::<Vec<_>>(), note_file).is_ok());
        }

        path.push(review_name);
        path.set_extension("toml");

        File::create(&path).expect("Could not create review!");
        fs::write(&path, contents).expect("Write to review toml failed!");
    }

    fn create_test_dir() -> PathBuf {
        let mut path = env::temp_dir();
        let mut app_name = std::env!("CARGO_CRATE_NAME").to_string();
        app_name.push_str("_repository_storage_test");
        path.push(app_name);

        path
    }

    fn create_test_repos(path: &PathBuf) {
        let review_helper_content = r#"name = "review_helper"
path = "/home/harry/workspace/review_helper"
first_commit = "9f89049b7f99682c48474d421ac126316adaed15"
base_branch = "main"
"#;

        let trackme_content = r#"name = "trackme"
path = "/home/harry/workspace/trackme"
first_commit = "5a99f0351a9dcbe5f2414e84e6f5bb9f617af33a"
base_branch = "main"
"#;
        create_repo(path.clone(), "review_helper", review_helper_content);

        let cool_feature_contents = r#"start_diff = "a261b7b"
end_diff = ""
"#;

        let fancy_ui_contents = r#"start_diff = "ed7811b"
end_diff = "a261b7b"

[[diff_files]]
is_reviewed = false
file_name = "bar.md"

[[diff_files]]
is_reviewed = true
file_name = "foo.md"
"#;

        create_review(path.clone(), "review_helper", "cool_feature", cool_feature_contents, Vec::new());

        let notes = vec![NoteStore {
            context: "foo/bar.cpp".to_string(),
            is_done: true,
            text: "fix bug".to_string(),
        }];
        create_review(path.clone(), "review_helper", "fancy_ui", fancy_ui_contents, notes);
        create_repo(path.clone(), "trackme", trackme_content);
    }

    #[serial]
    #[test]
    fn test_loading_repositories() {
        struct Context(PathBuf);
        impl Drop for Context {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let context = Context(create_test_dir());
        create_test_repos(&context.0);

        let repository_storage = ReviewHelperFileStorage::new(context.0.clone());
        let result = repository_storage.load_repositories();
        assert!(result.is_ok());

        let repositories = result.unwrap_or_default();

        let expected_repository = vec![
            RepositoryStore {
                path: PathBuf::from("/home/harry/workspace/review_helper"),
                first_commit: "9f89049b7f99682c48474d421ac126316adaed15".to_string(),
                name: "review_helper".into(),
                base_branch: "main".to_string(),
            },
            RepositoryStore {
                path: PathBuf::from("/home/harry/workspace/trackme"),
                first_commit: "5a99f0351a9dcbe5f2414e84e6f5bb9f617af33a".to_string(),
                name: "trackme".into(),
                base_branch: "main".to_string(),
            },
        ];

        for expected_repository in &expected_repository {
            assert!(repositories.iter().any(|r| r == expected_repository));
        }
    }

    #[serial]
    #[test]
    fn test_saving_repository() {
        struct Context(PathBuf);
        impl Drop for Context {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let context = Context(create_test_dir());
        let repository_storage = ReviewHelperFileStorage::new(context.0.clone());

        let repository_store = RepositoryStore {
            path: PathBuf::from("/home/harry/workspace/review_helper"),
            name: "review_helper".into(),
            first_commit: "9f89049b7f99682c48474d421ac126316adaed15".to_string(),
            base_branch: "main".to_string(),
        };
        let expected_repository_store = repository_store.clone();

        let result = repository_storage.save_repository(&repository_store);
        assert!(result.is_ok());

        let load_result = repository_storage.load_repositories();
        assert!(load_result.is_ok());

        assert_eq!(load_result.unwrap_or_default(), vec![expected_repository_store]);
    }

    #[serial]
    #[test]
    fn test_loading_review_names() {
        struct Context(PathBuf);
        impl Drop for Context {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let context = Context(create_test_dir());
        create_test_repos(&context.0);

        let repository_storage = ReviewHelperFileStorage::new(context.0.clone());
        let result = repository_storage.load_review_names(&"review_helper".into());
        assert!(result.is_ok());

        let current_names = result.unwrap_or_default();
        let expected_names = HashSet::from([ReviewName::from("cool_feature"), ReviewName::from("fancy_ui")]);

        assert_eq!(expected_names.len(), current_names.len());
        for name in current_names {
            assert!(expected_names.contains(&name));
        }
    }
    #[serial]
    #[test]
    fn test_loading_review() {
        struct Context(PathBuf);
        impl Drop for Context {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let context = Context(create_test_dir());
        create_test_repos(&context.0);

        let repository_storage = ReviewHelperFileStorage::new(context.0.clone());
        let review_result = repository_storage.load_review(&RepositoryName::from("review_helper"), &ReviewName::from("fancy_ui"));
        assert!(review_result.is_ok());

        let expected_file_diffs = vec![
            FileDiffStore {
                file_path: PathBuf::from("bar.md"),
                is_reviewed: false,
            },
            FileDiffStore {
                file_path: PathBuf::from("foo.md"),
                is_reviewed: true,
            },
        ];
        assert!(review_result.as_ref().unwrap().is_some());
        let review = review_result.unwrap_or_default().unwrap_or_default();

        let expected_diff_range = DiffRangeStore {
            start: "ed7811b".to_string(),
            end: "a261b7b".to_string(),
        };
        assert_eq!(review.diff_range, expected_diff_range);

        assert_eq!(review.file_diff_list.len(), expected_file_diffs.len());

        for file_diff_item in review.file_diff_list {
            assert!(expected_file_diffs.contains(&file_diff_item));
        }

        assert_eq!(review.notes.len(), 1);
        assert_eq!(
            review.notes[0],
            NoteStore {
                context: "foo/bar.cpp".to_string(),
                is_done: true,
                text: "fix bug".to_string(),
            }
        );

        let review_result = repository_storage.load_review(&RepositoryName::from("review_helper"), &ReviewName::from("cool_feature"));
        assert!(review_result.is_ok());
    }
    #[serial]
    #[test]
    fn test_storing_review() {
        struct Context(PathBuf);
        impl Drop for Context {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let context = Context(create_test_dir());
        let repository_storage = ReviewHelperFileStorage::new(context.0.clone());

        let repository_name = RepositoryName::from("review_helper");

        let repository_store = RepositoryStore {
            path: PathBuf::from("/home/harry/workspace/review_helper"),
            name: repository_name.clone(),
            first_commit: "9f89049b7f99682c48474d421ac126316adaed15".to_string(),
            base_branch: "main".to_string(),
        };

        let _result = repository_storage.save_repository(&repository_store);

        let review_store = ReviewStore {
            diff_range: DiffRangeStore {
                start: "0xfoo".to_string(),
                end: "".to_string(),
            },
            file_diff_list: vec![FileDiffStore {
                file_path: PathBuf::from("/foo/bar.txt"),
                is_reviewed: true,
            }],
            notes: vec![NoteStore {
                context: "/foo/bar.txt".to_string(),
                text: "Fix bug".to_string(),
                is_done: true,
            }],
        };
        let review_name = ReviewName::from("fancy_stuff");
        let result = repository_storage.save_review_notes(&repository_name, &review_name, &review_store.notes.iter().collect::<Vec<_>>());
        assert!(result.is_ok());

        let result = repository_storage.save_review_file_diffs(
            &repository_name,
            &review_name,
            &review_store.diff_range,
            &review_store.file_diff_list.iter().collect::<Vec<_>>(),
        );
        assert!(result.is_ok());

        let result = repository_storage.load_review(&repository_name, &review_name);
        assert!(result.is_ok());
        let opt_review = result.unwrap_or_default();
        assert!(opt_review.is_some());
        let current_review = opt_review.unwrap_or_default();
        assert_eq!(current_review, review_store);
    }
}
