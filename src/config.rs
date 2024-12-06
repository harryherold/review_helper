use std::fs;
use std::path::PathBuf;

use serde_derive::{Deserialize, Serialize};

use toml;

use crate::repository::Repository;

#[derive(Serialize, Deserialize, Debug)]
pub struct DiffFile {
    pub is_reviewed: bool,
    pub file_name: String,
}

impl DiffFile {
    pub fn new(is_reviewed: bool, file: String) -> DiffFile {
        DiffFile { is_reviewed, file_name: file }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub version: u32,
    pub start_diff: String,
    pub end_diff: String,
    pub first_commit: String,
    pub repo_path: String,
    pub diff_files: Vec<DiffFile>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            version: 1,
            start_diff: "".into(),
            end_diff: "".into(),
            first_commit: "".into(),
            repo_path: "".into(),
            diff_files: Vec::new(),
        }
    }
    pub fn save(path: &PathBuf, config: &Config) -> anyhow::Result<()> {
        let contents = toml::to_string(config)?;
        fs::write(path, contents).map_err(|e| anyhow::format_err!(e.to_string()))
    }
    pub fn read_from(path: &PathBuf) -> anyhow::Result<Config> {
        let file_content = fs::read_to_string(path)?;

        let mut config: Config = toml::from_str(&file_content)?;

        if config.version != 1 {
            return Err(anyhow::format_err!("Incorrect config version!"));
        }

        let path = PathBuf::from(&config.repo_path);
        if !path.exists() || !Repository::is_repo_valid(&path, Some(&config.first_commit))? {
            config.repo_path.clear();
        }

        Ok(config)
    }
}
