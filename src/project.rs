use std::path::PathBuf;

use native_dialog::FileDialog;

extern crate ini;
use ini::Ini;

use crate::notes::Notes;
use crate::repository::Repository;

pub struct Project {
    path: PathBuf,
    pub repository: Repository,
    pub notes: Notes,
}

impl Project {
    pub fn new() -> Project {
        Project {
            path: PathBuf::new(),
            repository: Repository::new(),
            notes: Notes::new(),
        }
    }
    pub fn open2(path: PathBuf) -> Result<Project, anyhow::Error> {
        let project_config = ProjectConfig::read_from(&path)?;
        project_config.check()?;

        Ok(Project {
            path: path,
            repository: Repository::new(),
            notes: Notes::new(),
        })
        // self.repository.set_path(project_config.repo_path.into());
        // self.path = path;
        // Ok(self.path.to_str().unwrap())
    }

    pub fn open(&mut self) -> Result<&str, anyhow::Error> {
        let path_option = FileDialog::new().add_filter("Ini project file", &["ini"]).show_open_single_file()?;
        if path_option.is_none() {
            return Ok("");
        }
        let path = path_option.unwrap();
        let project_config = ProjectConfig::read_from(&path)?;
        project_config.check()?;

        self.repository.set_path(project_config.repo_path.into());

        self.path = path;

        Ok(self.path.to_str().unwrap())
    }
    pub fn create(&mut self, path: PathBuf) {
        self.path = path;
    }
}

#[derive(Debug)]
struct ProjectConfig {
    version: f32,
    start_diff: String,
    end_diff: String,
    first_commit: String,
    repo_path: String,
}

impl ProjectConfig {
    fn read_from(path: &PathBuf) -> Result<ProjectConfig, anyhow::Error> {
        let config = Ini::load_from_file(path)?;
        let section = config.section(None::<String>).unwrap();

        let read_string = |field: &str| -> Result<&str, anyhow::Error> {
            match section.get(field) {
                None => Err(anyhow::format_err!("Field not found in ini file!")),
                Some(value) => Ok(value),
            }
        };

        Ok(ProjectConfig {
            start_diff: read_string("start-diff")?.into(),
            end_diff: read_string("end-diff")?.into(),
            first_commit: read_string("first-commit")?.into(),
            repo_path: read_string("repo-path")?.into(),
            version: read_string("rt-version")?.parse::<f32>()?,
        })
    }
    fn check(&self) -> Result<(), anyhow::Error> {
        if self.version != 0.1 {
            return Err(anyhow::format_err!("Incorret config version!"));
        }
        let path = PathBuf::from(&self.repo_path);
        if !path.exists() {
            return Err(anyhow::format_err!("Config path does not exists"));
        }
        if Repository::is_repo_valid(&path, Some(&self.first_commit))? {
            return Ok(());
        }
        Err(anyhow::format_err!("Repository is not valid!"))
    }
}
