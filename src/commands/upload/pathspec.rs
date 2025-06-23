use std::sync::Arc;

use anyhow::Result;
use gix::{Pathspec, Repository, bstr::BStr, fs::FileSnapshot, index::File};

pub fn pathspec_from_pattern<'repo>(
    repo: &'repo Repository,
    patterns: impl IntoIterator<Item = impl AsRef<BStr>>,
) -> Result<(Pathspec<'repo>, Arc<FileSnapshot<File>>)> {
    let state = repo.index()?;

    let spec = repo.pathspec(
        false,
        patterns,
        false,
        &state,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?;

    Ok((spec, state))
}
