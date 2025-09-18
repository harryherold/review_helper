use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use slint::{ComponentHandle, Weak};

use crate::commit_proxy_model::CommitProxyModel;
use crate::project::Project;
use crate::ui;
use crate::{app_config, git_utils};

pub fn async_query_commits(repo_path: &PathBuf, commit_proxy_model: Rc<RefCell<CommitProxyModel>>) {
    if !repo_path.exists() {
        return;
    }
    let path = repo_path.clone();
    slint::spawn_local(async move {
        let commits = tokio::spawn(async move { git_utils::query_commits(&path).expect("Could not query commits!") })
            .await
            .expect("tokio spawn query_commits failed!");
        commit_proxy_model.borrow_mut().set_commits(commits);
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
    let path_option = {
        let p = project.borrow();
        match p.repository.path.as_ref() {
            None => None,
            Some(path) => Some(path.clone()),
        }
    };
    if path_option.is_none() {
        return;
    }
    slint::spawn_local(async move {
        let (start, end) = {
            let p = project.borrow();
            let (s, e) = p.repository.diff_range();
            (s.to_string(), e.to_string())
        };

        let result = tokio::spawn(async move { git_utils::diff_git_repo(&path_option.expect("Path not available!"), &start, &end) })
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

pub fn async_query_diff_tools(app_config: Rc<RefCell<app_config::AppConfig>>, ui_weak: Weak<ui::AppWindow>) {
    slint::spawn_local(async move {
        let result = tokio::spawn(async move { git_utils::query_diff_tools() })
            .await
            .expect("tokio spawn query_diff_tools failed!");
        match result {
            Err(e) => eprintln!("Error on quering diff tools: {}", e.to_string()),
            Ok(diff_tools) => {
                let mut app_config = app_config.borrow_mut();
                app_config.set_diff_tools(&diff_tools);

                let ui = ui_weak.unwrap();

                let diff_tool = ui.global::<ui::AppConfig>().get_diff_tool().to_string();

                if let Some(index) = diff_tools.iter().position(|v| *v == diff_tool) {
                    ui.global::<ui::AppConfig>().set_difftool_index(index as i32);
                }
            }
        }
    })
    .expect("async_query_diff_tools: spawn_local failed!");
}
