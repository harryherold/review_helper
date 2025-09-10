use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;

use crate::project::Project;
use crate::git_utils;

pub fn async_query_commits(project: Rc<RefCell<Project>>) {
    if project.borrow().repository.repository_path().is_none() {
        return;
    }
    slint::spawn_local(async move {
        let path = {
            let p = project.borrow();
            let path_str = p.repository.repository_path().expect("No repository path available!");
            PathBuf::from(path_str)
        };
        let commits = tokio::spawn(async move { git_utils::query_commits(&path).expect("Could not query commits!") })
            .await
            .expect("tokio spawn query_commits failed!");
        let mut p = project.borrow_mut();
        p.repository.set_commit_history(commits);
    })
    .expect("spawn_local failed!");
}