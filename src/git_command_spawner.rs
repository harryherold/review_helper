use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Weak};

use crate::git_utils;
use crate::project::Project;
use crate::ui;

pub fn async_query_commits(project: Rc<RefCell<Project>>) {
    if project.borrow().repository.repository_path().is_none() {
        return;
    }
    slint::spawn_local(async move {
        let path = {
            let p = project.borrow();
            let path_str = p.repository.repository_path().unwrap();
            PathBuf::from(path_str)
        };
        let commits = tokio::spawn(async move { git_utils::query_commits(&path).expect("Could not query commits!") })
            .await
            .expect("tokio spawn query_commits failed!");
        let mut p = project.borrow_mut();
        p.repository.set_commit_history(commits);
    })
    .expect("async_query_commits: spawn_local failed!");
}

pub fn async_diff_file(repo_path: &PathBuf, start_commit: &str, end_commit: &str, file: &str, diff_tool: &str) -> anyhow::Result<()> {
    let path = repo_path.clone();
    let start = start_commit.to_string();
    let end = end_commit.to_string();
    let file = file.to_string();
    let tool = diff_tool.to_string();
    slint::spawn_local(async move {
        tokio::spawn(async move {
            git_utils::diff_file(&path, &start, &end, &file, &tool).expect("Could not diff files!");
        })
        .await
        .expect("tokio spawn diff_file failed!");
    })
    .expect("async_diff_file: spawn_local failed!");
    Ok(())
}

pub fn async_diff_repository(project: Rc<RefCell<Project>>, ui_weak: Weak<ui::AppWindow>) {
    if project.borrow().repository.repository_path().is_none() {
        return;
    }
    slint::spawn_local(async move {
        let path = {
            let p = project.borrow();
            let path_str = p.repository.repository_path().unwrap();
            PathBuf::from(path_str)
        };
        let (start, end) = {
            let p = project.borrow();
            let (s, e) = p.repository.diff_range();
            (s.to_string(), e.to_string())
        };

        let result = tokio::spawn(async move { git_utils::diff_git_repo(&path, &start, &end) })
            .await
            .expect("tokio spawn diff_git_repo failed!");
        if let Err(error) = result {
            eprintln!("Error on diffing repo: {}", error.to_string());
            return;
        }
        let mut project = project.borrow_mut();
        project.repository.merge_file_diff_map(result.unwrap());

        let ui = ui_weak.unwrap();
        let statistics = project.repository.statistics();

        ui.global::<ui::OverallDiffStats>().set_added_lines(statistics.added_lines as i32);
        ui.global::<ui::OverallDiffStats>().set_removed_lines(statistics.removed_lines as i32);
    })
    .expect("async_diff_repository: spawn_local failed!");
}
