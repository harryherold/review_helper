use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use std::{path::PathBuf, rc::Rc};

use slint::{Model, ModelRc};

use crate::git_utils::ChangeType;
use crate::id_model::IdModel;
use crate::{project_config::ProjectConfig, git_utils, ui};

pub struct Repository {
    path: PathBuf,
    current_diff: Diff,
}

fn diff_file_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

type DiffModelRc = Rc<IdModel<ui::DiffFileItem>>;
struct Diff {
    start_commit: String,
    end_commit: String,
    file_diff_model: DiffModelRc,
}

impl Diff {
    pub fn new() -> Diff {
        let file_diff_model = Rc::new(IdModel::<ui::DiffFileItem>::default());
        Diff {
            start_commit: String::new(),
            end_commit: String::new(),
            file_diff_model: file_diff_model.clone(),
        }
    }
    fn id_to_index(&self, id: i32) -> Option<usize> {
        for (idx, item) in self.file_diff_model.iter().enumerate() {
            if item.id == id {
                return Some(idx);
            }
        }
        None
    }
}

impl Repository {
    pub fn new() -> Repository {
        Repository {
            path: "".into(),
            current_diff: Diff::new(),
        }
    }

    pub fn from_project_config(project_config: &ProjectConfig) -> anyhow::Result<Repository> {
        let mut repo = Repository {
            path: PathBuf::from(project_config.repo_path.to_string()),
            current_diff: Diff::new(),
        };
        repo.current_diff.start_commit = project_config.start_diff.clone();
        repo.current_diff.end_commit = project_config.end_diff.clone();

        repo.current_diff.file_diff_model.clear();
        for diff_file in &project_config.diff_files {
            let id = diff_file_id();

            repo.current_diff.file_diff_model.add(
                id,
                ui::DiffFileItem {
                    id: id as i32,
                    text: diff_file.file_name.to_owned().into(),
                    is_reviewed: diff_file.is_reviewed,
                    added_lines: -1,
                    removed_lines: -1,
                    change_type: ui::ChangeType::Invalid,
                },
            )
        }
        Ok(repo)
    }

    pub fn is_repo_valid(path: &PathBuf, opt_first_commit: Option<&str>) -> anyhow::Result<bool> {
        if !git_utils::is_git_repo(path) {
            return Ok(false);
        }
        match opt_first_commit {
            None => Ok(true),
            Some(first_commit) => Ok(git_utils::repo_contains_commit(path, first_commit)?),
        }
    }

    pub fn repository_path(&self) -> Option<&str> {
        if !self.path.exists() {
            None
        } else {
            self.path.to_str()
        }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = path;
    }

    pub fn diff_repository(&mut self, start_commit: &str, end_commit: &str) -> anyhow::Result<()> {
        self.current_diff.start_commit = start_commit.to_string();
        self.current_diff.end_commit = end_commit.to_string();

        let files_stats = git_utils::diff_git_repo(&self.path, &start_commit, &end_commit)?;

        let mut old_files: HashSet<String> = HashSet::new();
        let mut file_index_map: HashMap<String, usize> = HashMap::new();

        self.current_diff
            .file_diff_model
            .iter()
            .map(|item| (item.id as usize, item.text.to_string()))
            .for_each(|(id, file)| {
                file_index_map.insert(file.to_owned(), id);
                old_files.insert(file);
            });

        let diff_files: HashSet<String> = files_stats.keys().cloned().collect();

        let change_type_to_ui = |change_type: &ChangeType| match change_type {
            git_utils::ChangeType::Added => ui::ChangeType::Added,
            git_utils::ChangeType::Broken => ui::ChangeType::Broken,
            git_utils::ChangeType::Copied => ui::ChangeType::Copied,
            git_utils::ChangeType::Deleted => ui::ChangeType::Deleted,
            git_utils::ChangeType::Modified => ui::ChangeType::Modified,
            git_utils::ChangeType::Renamed => ui::ChangeType::Renamed,
            git_utils::ChangeType::TypChanged => ui::ChangeType::TypChanged,
            git_utils::ChangeType::Unmerged => ui::ChangeType::Unmerged,
            git_utils::ChangeType::Unknown => ui::ChangeType::Unknown,
            git_utils::ChangeType::Invalid => ui::ChangeType::Invalid,
        };

        let update_item = |mut item: ui::DiffFileItem| {
            let file_stat = files_stats.get(item.text.as_str()).unwrap();

            if item.added_lines != file_stat.added_lines as i32 || item.removed_lines != file_stat.removed_lines as i32 {
                item.added_lines = file_stat.added_lines as i32;
                item.removed_lines = file_stat.removed_lines as i32;
                item.change_type = change_type_to_ui(&file_stat.change_type);
                self.current_diff.file_diff_model.update(item.id as usize, item);
            }
        };
        let add_item = |file: &String| {
            let file_stat = files_stats.get(file).unwrap();
            let id = diff_file_id();
            self.current_diff.file_diff_model.add(
                id,
                ui::DiffFileItem {
                    id: id as i32,
                    text: file.into(),
                    is_reviewed: false,
                    added_lines: file_stat.added_lines as i32,
                    removed_lines: file_stat.removed_lines as i32,
                    change_type: change_type_to_ui(&file_stat.change_type),
                },
            );
        };

        if diff_files == old_files {
            for item in self.current_diff.file_diff_model.iter() {
                update_item(item);
            }
            return Ok(());
        } else if diff_files.is_disjoint(&old_files) {
            self.current_diff.file_diff_model.clear();
            diff_files.iter().for_each(add_item);
        } else {
            let modified_files = old_files.intersection(&diff_files).collect::<HashSet<&String>>();
            for modified_file in modified_files {
                let index = file_index_map.get(modified_file).expect("Modified files should not be deleted!");

                if let Some(item) = self.current_diff.file_diff_model.row_data(*index) {
                    update_item(item);
                }
            }

            let deleted_files: HashSet<&String> = old_files.difference(&diff_files).collect();
            for deleted_file in deleted_files {
                let delete_item = self.current_diff.file_diff_model.iter().enumerate().find(|(_, item)| item.text == deleted_file);
                if let Some((row, _)) = delete_item {
                    self.current_diff.file_diff_model.remove(row);
                }
            }
            let new_files: HashSet<&String> = diff_files.difference(&old_files).collect();
            new_files.into_iter().for_each(add_item);
        }

        Ok(())
    }

    pub fn toggle_file_is_reviewed(&mut self, id: usize) {
        if let Some(mut item) = self.current_diff.file_diff_model.get(id) {
            item.is_reviewed = !item.is_reviewed;
            self.current_diff.file_diff_model.update(id, item);
        }
    }

    pub fn diff_file(&self, id: i32) -> anyhow::Result<()> {
        let index = self.current_diff.id_to_index(id).expect("id-index-mapping is broken!");
        match self.current_diff.file_diff_model.row_data(index as usize) {
            None => Err(anyhow::format_err!("Could not found file in model!")),
            Some(file_item) => git_utils::diff_file(&self.path, &self.current_diff.start_commit, &self.current_diff.end_commit, &file_item.text),
        }
    }

    pub fn file_diff_model(&self) -> ModelRc<ui::DiffFileItem> {
        self.current_diff.file_diff_model.clone().into()
    }

    pub fn diff_range(&self) -> (&str, &str) {
        (&self.current_diff.start_commit, &self.current_diff.end_commit)
    }
}
