use std::path::PathBuf;

use crate::config::Config;
use crate::notes::Notes;
use crate::repository::Repository;

pub struct Project {
    path: PathBuf,
    pub repository: Repository,
    pub notes: Notes,
}

impl Project {
    pub fn new() -> Project {
        let config = Config::new();
        Project {
            path: PathBuf::new(),
            repository: Repository::new(&config),
            notes: Notes::new(None),
        }
    }
    pub fn open(config: &Config) -> Result<Project, anyhow::Error> {
        Ok(Project {
            path: config.project_file.clone(),
            repository: Repository::new(&config),
            notes: Notes::new(config.project_file.parent()),
        })
    }
}
