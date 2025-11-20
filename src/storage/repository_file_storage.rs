use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use toml::{Table, Value};

use crate::storage::repository_storage::ReviewName;
use crate::storage::{RepositoryName, RepositoryStore, ReviewHelperStorage};

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

    fn save_repository(&self, repository_store: RepositoryStore) -> anyhow::Result<()> {
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
        table.insert("first_commit".to_string(), Value::String(repository_store.first_commit));
        table.insert("name".to_string(), Value::String(String::from(&repository_store.name)));
        table.insert("base_branch".to_string(), Value::String(String::from(&repository_store.base_branch)));

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
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use crate::storage::repository_storage::{RepositoryStore, ReviewName};

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

    fn create_review(mut path: PathBuf, repository_name: &str, review_name: &str, contents: &str) {
        path.push(repository_name);
        path.push(review_name);
        if !path.exists() {
            assert!(fs::create_dir_all(&path).is_ok());
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

[[diff_files]]
is_reviewed = false
file_name = "Changes.md"
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

        create_review(path.clone(), "review_helper", "cool_feature", cool_feature_contents);
        create_review(path.clone(), "review_helper", "fancy_ui", fancy_ui_contents);
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

        let result = repository_storage.save_repository(repository_store);
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
}
