use std::path::Path;

use git2::{DiffLineType, DiffOptions, Error, Repository, Tree};

#[derive(Debug, PartialEq)]
pub enum LineType {
    Added,
    Removed,
    Unchanged,
}

pub struct GitDiffLine {
    pub old_line_no: i32,
    pub new_line_no: i32,
    pub status: LineType,
    pub line: String,
}

pub struct GitRepo {
    repository: Repository,
}

impl GitRepo {
    pub fn create(repository_path: &Path) -> Result<Self, Error> {
        Ok(GitRepo {
            repository: Repository::open(repository_path)?,
        })
    }
    pub fn diff(&self, from: &str, to: Option<&str>, file: &str) -> Result<Vec<GitDiffLine>, Error> {
        let mut diff_options = DiffOptions::new();
        diff_options.context_lines(u32::MAX);

        diff_options.pathspec(file);

        let from_tree = self.tree_to_treeish(from)?;
        let diff = if let Some(to) = to {
            let to_tree = self.tree_to_treeish(to)?;
            self.repository
                .diff_tree_to_tree(from_tree.as_ref(), to_tree.as_ref(), Some(&mut diff_options))?
        } else {
            self.repository.diff_tree_to_workdir(from_tree.as_ref(), Some(&mut diff_options))?
        };

        let mut changed_lines = Vec::new();
        let mut old_line_counter = 0;
        let mut new_line_counter = 0;

        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let status = match line.origin_value() {
                DiffLineType::Addition => LineType::Added,
                DiffLineType::Deletion => LineType::Removed,
                DiffLineType::Context => LineType::Unchanged,
                _ => return true,
            };
            let old_no = if status == LineType::Added {
                -1
            } else {
                old_line_counter += 1;
                old_line_counter
            };
            let new_no = if status == LineType::Removed {
                -1
            } else {
                new_line_counter += 1;
                new_line_counter
            };

            let content = std::str::from_utf8(line.content())
                .unwrap_or("")
                .trim_end_matches(['\r', '\n']) // Zeilenumbrüche entfernen
                .to_string();

            changed_lines.push(GitDiffLine {
                old_line_no: old_no,
                new_line_no: new_no,
                status,
                line: content,
            });
            true
        })?;
        Ok(changed_lines)
    }

    fn tree_to_treeish(&self, arg: &str) -> Result<Option<Tree<'_>>, Error> {
        let obj = self.repository.revparse_single(arg)?;
        let tree = obj.peel_to_tree()?;
        Ok(Some(tree))
    }
}
