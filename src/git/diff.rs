//! Gix diffing utils

use anyhow::Result;
use gix::{
    Commit,
    diff::{Options, tree_with_rewrites::Change},
};

pub fn diff_from_head(commit: Commit<'_>) -> Result<Vec<Change>> {
    let repo = commit.repo;

    let remote_tree = commit.tree()?;
    let local_tree = repo.head_tree()?;

    let options = Options::default().with_rewrites(None);

    Ok(repo.diff_tree_to_tree(&remote_tree, &local_tree, Some(options))?)
}
