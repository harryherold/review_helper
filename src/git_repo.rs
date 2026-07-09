use std::path::Path;

use git2::{DiffLineType, DiffOptions, Error, Repository, Tree};

#[derive(Debug, PartialEq)]
pub enum LineType {
    Added,
    Removed,
    Unchanged,
}

pub struct GitDiffLine {
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
    pub fn diff(&self, from: &str, to: &str, file: &str) -> Result<Vec<GitDiffLine>, Error> {
        let from_tree = self.tree_to_treeish(Some(&from.to_string()))?;
        let to_tree = self.tree_to_treeish(Some(&to.to_string()))?;

        let mut diff_options = DiffOptions::new();
        diff_options.context_lines(u32::MAX);

        diff_options.pathspec(file);

        let diff = self.repository.diff_tree_to_tree(
            from_tree.as_ref(),
            to_tree.as_ref(),
            Some(&mut diff_options),
        )?;

        let mut changed_lines = Vec::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            let status = match line.origin_value() {
                DiffLineType::Addition => LineType::Added,
                DiffLineType::Deletion => LineType::Removed,
                DiffLineType::Context => LineType::Unchanged,
                _ => return true,
            };

            let content = std::str::from_utf8(line.content())
                .unwrap_or("")
                .trim_end_matches(['\r', '\n']) // Zeilenumbrüche entfernen
                .to_string();

            changed_lines.push(GitDiffLine {
                status,
                line: content,
            });
            true
        })?;
        Ok(changed_lines)
    }

    fn tree_to_treeish(&self, arg: Option<&String>) -> Result<Option<Tree<'_>>, Error> {
        let arg = match arg {
            Some(s) => s,
            None => return Ok(None),
        };
        let obj = self.repository.revparse_single(arg)?;
        let tree = obj.peel_to_tree()?;
        Ok(Some(tree))
    }
}
