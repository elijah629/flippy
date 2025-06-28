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

    let options = Options::default().with_rewrites(None); // Simpler logic when
    // converting changes into operations, it is also cheaper.

    /*options = *options.track_rewrites(Some(Rewrites {
        copies: Some(Copies {
            source: gix::diff::rewrites::CopySource::FromSetOfModifiedFiles,
            percentage: None,
        }),
        percentage: None,
        limit: usize::MAX,
        track_empty: false,
    }));*/
    // options = *options.track_filename(); // Faster but not accurate

    Ok(repo.diff_tree_to_tree(&remote_tree, &local_tree, Some(options))?)
}
