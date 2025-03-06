use std::path::PathBuf;

use slint::Model;

use crate::project_config;
use crate::git_utils;
use crate::notes::Notes;
use crate::repository::Repository;

use project_config::ProjectConfig;

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
    pub fn from_config(project_file: &PathBuf, config: ProjectConfig) -> anyhow::Result<Project> {
        let project_folder = project_file.parent().expect("Cannot determine parent!");
        Ok(Project {
            path: project_file.to_owned(),
            repository: Repository::from_project_config(&config)?,
            notes: Notes::new(project_folder)?,
        })
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let mut config = ProjectConfig::new();
        if let Some(repo_path) = self.repository.repository_path() {
            config.repo_path = repo_path.to_string();
            config.first_commit = git_utils::first_commit(&PathBuf::from(repo_path))?;
        }
        let (start_commit, end_commit) = self.repository.diff_range();
        config.start_diff = start_commit.to_string();
        config.end_diff = end_commit.to_string();

        for diff_file_item in self.repository.file_diff_model().iter() {
            config
                .diff_files
                .push(project_config::DiffFile::new(diff_file_item.is_reviewed, diff_file_item.text.into()));
        }
        ProjectConfig::save(&self.path, &config)?;
        self.notes.save()
    }
}
