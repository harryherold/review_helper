use std::path::PathBuf;

use crate::config::Config;
use crate::git_utils;
use crate::notes::Notes;
use crate::repository::Repository;

pub struct Project {
    path: PathBuf,
    pub repository: Repository,
    pub notes: Notes,
}

impl Project {
    pub fn new(path: &PathBuf) -> anyhow::Result<Project> {
        Ok(Project {
            path: path.clone(),
            repository: Repository::new(),
            notes: Notes::new(path.parent().expect("Cannot determine parent!"))?,
        })
    }
    pub fn default() -> Project {
        Project {
            path: PathBuf::new(),
            repository: Repository::new(),
            notes: Notes::default(),
        }
    }
    pub fn from_config(config: Config) -> Result<Project, anyhow::Error> {
        let project_folder = config.project_file.parent().expect("Cannot determine parent!");
        Ok(Project {
            path: config.project_file.clone(),
            repository: Repository::from_config(&config),
            notes: Notes::new(project_folder)?,
        })
    }
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let mut config = Config::new();
        config.project_file = self.path.clone();
        if let Some(repo_path) = self.repository.repository_path() {
            config.repo_path = repo_path.to_string();
            config.first_commit = git_utils::first_commit(&PathBuf::from(repo_path))?;
        }
        let (start_commit, end_commit) = self.repository.diff_range();
        config.start_diff = start_commit.to_string();
        config.end_diff = end_commit.to_string();

        config.save()?;
        self.notes.save()
    }
}
