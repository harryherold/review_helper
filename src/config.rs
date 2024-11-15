use std::path::PathBuf;

extern crate ini;
use ini::Ini;

use crate::repository::Repository;

#[derive(Debug)]
pub struct Config {
    pub version: f32,
    pub start_diff: String,
    pub end_diff: String,
    pub first_commit: String,
    pub repo_path: String,
    pub project_file: PathBuf,
}

impl Config {
    pub fn new() -> Config {
        Config {
            version: 0.1,
            start_diff: "".into(),
            end_diff: "".into(),
            first_commit: "".into(),
            repo_path: "".into(),
            project_file: "".into(),
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let mut config = Ini::new();
        config.with_section(None::<String>).set("rt-version", self.version.to_string());
        config.with_section(None::<String>).set("repo-path", self.repo_path.clone());
        config.with_section(None::<String>).set("first-commit", self.first_commit.clone());
        config.with_section(None::<String>).set("start-diff", self.start_diff.clone());
        config.with_section(None::<String>).set("end-diff", self.end_diff.clone());

        config.write_to_file(self.project_file.clone())?;
        Ok(())
    }
    pub fn read_from(path: &PathBuf) -> anyhow::Result<Config> {
        let config = Ini::load_from_file(path)?;
        let section = config.section(None::<String>).unwrap();

        let read_string = |field: &str| -> Result<&str, anyhow::Error> {
            match section.get(field) {
                None => Err(anyhow::format_err!("Field not found in ini file!")),
                Some(value) => Ok(value),
            }
        };

        let version = read_string("rt-version")?.parse::<f32>()?;
        if version != 0.1 {
            return Err(anyhow::format_err!("Incorrect config version!"));
        }

        let first_commit: &str = read_string("first-commit")?.into();
        let read_repo_path = || -> Result<&str, anyhow::Error> {
            let repo_path = read_string("repo-path")?.into();
            let path = PathBuf::from(&repo_path);

            if path.exists() && Repository::is_repo_valid(&path, Some(&first_commit))? {
                return Ok(repo_path);
            }
            return Ok("");
        };

        Ok(Config {
            version: version,
            start_diff: read_string("start-diff")?.into(),
            end_diff: read_string("end-diff")?.into(),
            first_commit: first_commit.into(),
            repo_path: read_repo_path()?.into(),
            project_file: path.into(),
        })
    }
}
