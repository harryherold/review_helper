use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::AtomicUsize;
use std::{path::PathBuf, rc::Rc};

use slint::{Model, ModelRc, VecModel};

use crate::git_command_spawner;
use crate::git_utils::ChangeType;
use crate::id_model::{IdModel, IdModelChange};
use crate::ui::OverallStat;
use crate::{git_utils, project_config::ProjectConfig, ui};

pub struct Repository {
    pub path: Option<PathBuf>,
    current_diff: Diff,
}

fn diff_file_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(1);
    COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

type DiffStatModel = Rc<VecModel<ui::OverallStat>>;
pub struct DiffStatistics {
    pub added_lines: u32,
    pub removed_lines: u32,
    pub statistics_model: DiffStatModel,
}

impl DiffStatistics {
    fn new() -> Self {
        DiffStatistics {
            added_lines: 0,
            removed_lines: 0,
            statistics_model: Rc::new(VecModel::<ui::OverallStat>::default()),
        }
    }
    fn clear(&mut self) {
        self.statistics_model.clear();
        self.added_lines = 0;
        self.removed_lines = 0;
    }
}

type DiffModelRc = Rc<IdModel<ui::DiffFileItem>>;
struct Diff {
    start_commit: String,
    end_commit: String,
    file_diff_model: DiffModelRc,
    statistics: DiffStatistics,
}

impl Diff {
    pub fn new() -> Diff {
        Diff {
            start_commit: String::new(),
            end_commit: String::new(),
            file_diff_model: Rc::new(IdModel::<ui::DiffFileItem>::default()),
            statistics: DiffStatistics::new(),
        }
    }
}

impl Default for Repository {
    fn default() -> Self {
        Repository {
            path: None,
            current_diff: Diff::new(),
        }
    }
}

impl Repository {
    pub fn from_project_config(project_config: &ProjectConfig) -> anyhow::Result<Repository> {
        let mut repo = Self::default();
        if !project_config.repo_path.is_empty() {
            repo.set_path(PathBuf::from(project_config.repo_path.to_string()));
        }

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

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn merge_file_diff_map(&mut self, file_diff_map: git_utils::FileDiffMap) {
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

        let diff_files: HashSet<String> = file_diff_map.keys().cloned().collect();

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

        self.current_diff.statistics.clear();
        let mut change_type_map = BTreeMap::<ChangeType, u32>::new();
        for file_stat in file_diff_map.values() {
            self.current_diff.statistics.added_lines += file_stat.added_lines;
            self.current_diff.statistics.removed_lines += file_stat.removed_lines;

            change_type_map
                .entry(file_stat.change_type.clone())
                .and_modify(|current| *current += 1)
                .or_insert(1);
        }
        for (change_type, count) in change_type_map {
            self.current_diff.statistics.statistics_model.push(OverallStat {
                change_type: change_type_to_ui(&change_type),
                count: count as i32,
            });
        }

        let update_item = |mut item: ui::DiffFileItem| {
            let file_stat_opt = file_diff_map.get(item.text.as_str());
            match file_stat_opt {
                Some(file_stat) => {
                    if item.added_lines != file_stat.added_lines as i32 || item.removed_lines != file_stat.removed_lines as i32 {
                        item.added_lines = file_stat.added_lines as i32;
                        item.removed_lines = file_stat.removed_lines as i32;
                        item.change_type = change_type_to_ui(&file_stat.change_type);
                        self.current_diff.file_diff_model.update(item.id as usize, item);
                    }
                }
                None => eprintln!("Error item not found! {}", &item.text),
            }
        };
        let add_item = |file: &String| {
            let file_stat = file_diff_map.get(file).unwrap();
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
            return;
        } else if diff_files.is_disjoint(&old_files) {
            self.current_diff.file_diff_model.clear();
            diff_files.iter().for_each(add_item);
        } else {
            let modified_files = old_files.intersection(&diff_files).collect::<HashSet<&String>>();
            for modified_file in modified_files {
                let id = file_index_map.get(modified_file).expect("Modified files should not be deleted!");

                if let Some(item) = self.current_diff.file_diff_model.get(*id) {
                    update_item(item);
                }
            }

            let deleted_files: HashSet<&String> = old_files.difference(&diff_files).collect();
            for deleted_file in deleted_files {
                let delete_item = self
                    .current_diff
                    .file_diff_model
                    .iter()
                    .find(|item| item.text == deleted_file)
                    .map(|item| item.id);
                if let Some(id) = delete_item {
                    self.current_diff.file_diff_model.remove(id as usize);
                }
            }
            let new_files: HashSet<&String> = diff_files.difference(&old_files).collect();
            new_files.into_iter().for_each(add_item);
        }
    }

    pub fn toggle_file_is_reviewed(&mut self, id: usize) {
        if let Some(mut item) = self.current_diff.file_diff_model.get(id) {
            item.is_reviewed = !item.is_reviewed;
            self.current_diff.file_diff_model.update(id, item);
        }
    }

    pub fn diff_file(&self, id: i32, diff_tool: &str) -> anyhow::Result<()> {
        if self.path.is_none() {
            return Err(anyhow::format_err!("Repository path not set!"));
        }
        let path = self.path.as_ref().unwrap();
        match self.current_diff.file_diff_model.get(id as usize) {
            None => panic!("Could not found file in model!"),
            Some(file_item) => git_command_spawner::async_diff_file(
                &path,
                &self.current_diff.start_commit,
                &self.current_diff.end_commit,
                &file_item.text,
                diff_tool,
            ),
        }
    }

    pub fn file_diff_model(&self) -> ModelRc<ui::DiffFileItem> {
        self.current_diff.file_diff_model.clone().into()
    }

    pub fn observe_file_diff_model<Observer: Fn(IdModelChange) + 'static>(&self, observer: Observer) {
        self.current_diff.file_diff_model.set_observer(observer);
    }

    pub fn set_diff_range(&mut self, range: (&str, &str)) {
        let (start, end) = range;
        self.current_diff.start_commit = start.to_string();
        self.current_diff.end_commit = end.to_string();
    }

    pub fn diff_range(&self) -> (&str, &str) {
        (&self.current_diff.start_commit, &self.current_diff.end_commit)
    }

    pub fn statistics(&self) -> &DiffStatistics {
        &self.current_diff.statistics
    }
}
