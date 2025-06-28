use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::walking_diff::tree::{RemoteTree, Tree};

#[derive(Debug)]
pub enum Op {
    /// Root dir
    Repo(PathBuf),
    /// src -> destination
    Mapping(String, &'static str),
    Copy(PathBuf),
    CreateDir(PathBuf),
    Remove(PathBuf),
}

pub trait DiffFn: FnMut(&Path, Option<u32>, usize, usize) -> Result<bool> {}
impl<T> DiffFn for T where T: FnMut(&Path, Option<u32>, usize, usize) -> Result<bool> {}

pub fn diff(
    local: &Tree,
    remote: &RemoteTree,
    ops: &mut Vec<Op>,
    different: impl DiffFn,
) -> Result<()> {
    let matches = prune_pass(local, remote, ops)?;
    creation_pass(local, remote, ops)?;
    update_pass(local, matches, different, ops)?;

    Ok(())
}

fn prune_pass(
    local: &Tree,
    remote: &RemoteTree,
    ops: &mut Vec<Op>,
) -> Result<Vec<(PathBuf, usize, usize, usize)>> {
    let mut matched = Vec::new();

    #[allow(clippy::too_many_arguments)]
    fn walk(
        local: &Tree,
        remote: &RemoteTree,
        local_idx: usize,
        remote_idx: usize,
        remote_parent: usize,
        path: &Path,
        ops: &mut Vec<Op>,
        matched: &mut Vec<(PathBuf, usize, usize, usize)>,
    ) -> Result<()> {
        // Record this node as unchanged
        matched.push((path.to_path_buf(), local_idx, remote_idx, remote_parent));

        let remote_node = &remote.nodes[remote_idx];
        let local_node = &local.nodes[local_idx];
        for (name, &r_child_idx) in &remote_node.children {
            let mut child_path = path.to_path_buf();
            child_path.push(name.as_ref());
            if let Some(&l_child_idx) = local_node.children.get(name) {
                // Both exist: recurse
                walk(
                    local,
                    remote,
                    l_child_idx,
                    r_child_idx,
                    remote_idx,
                    &child_path,
                    ops,
                    matched,
                )?;
            } else {
                // Remote-only: remove subtree
                ops.push(Op::Remove(child_path.strip_prefix("/")?.to_path_buf()));
            }
        }

        Ok(())
    }

    let root = PathBuf::from("/");
    walk(local, remote, 0, 0, 0, &root, ops, &mut matched)?;
    Ok(matched)
}

/// Emits operations to create a complete subtree rooted at local_idx and path.
fn emit_create_subtree(
    tree: &Tree,
    local_idx: usize,
    path: &PathBuf,
    ops: &mut Vec<Op>,
) -> Result<()> {
    let local_node = &tree.nodes[local_idx];
    if local_node.children.is_empty() {
        // File
        ops.push(Op::Copy(path.strip_prefix("/")?.to_path_buf()));
    } else {
        // Directory
        if path != &PathBuf::from("/") {
            ops.push(Op::CreateDir(path.strip_prefix("/")?.to_path_buf()));
        }
        for (name, &child_idx) in &local_node.children {
            let mut child_path = path.clone();
            child_path.push(name.as_ref());
            emit_create_subtree(tree, child_idx, &child_path, ops)?;
        }
    }

    Ok(())
}

fn creation_pass(local: &Tree, remote: &RemoteTree, ops: &mut Vec<Op>) -> Result<()> {
    fn walk(
        local: &Tree,
        remote: &RemoteTree,
        local_idx: usize,
        remote_idx_opt: Option<usize>,
        path: &PathBuf,
        ops: &mut Vec<Op>,
    ) -> Result<()> {
        if let Some(r_idx) = remote_idx_opt {
            // Both have directory: recurse
            let local_node = &local.nodes[local_idx];
            let remote_node = &remote.nodes[r_idx];
            for (name, &l_child_idx) in &local_node.children {
                let mut child_path = path.clone();

                child_path.push(name.as_ref());
                if let Some(&r_child_idx) = remote_node.children.get(name) {
                    walk(
                        local,
                        remote,
                        l_child_idx,
                        Some(r_child_idx),
                        &child_path,
                        ops,
                    )?;
                } else {
                    // Local-only: emit subtree
                    emit_create_subtree(local, l_child_idx, &child_path, ops)?;
                }
            }
        } else {
            // Remote missing: create entire subtree
            emit_create_subtree(local, local_idx, path, ops)?;
        }

        Ok(())
    }

    let root = PathBuf::from("/");
    walk(local, remote, 0, Some(0), &root, ops)?;

    Ok(())
}

fn update_pass(
    local: &Tree,
    matched: Vec<(PathBuf, usize, usize, usize)>,
    mut different: impl DiffFn,
    ops: &mut Vec<Op>,
) -> Result<()> {
    for (path, l_idx, r_idx, r_parent) in matched {
        let local_node = &local.nodes[l_idx];
        if local_node.children.is_empty() && different(&path, local_node.size, r_idx, r_parent)? {
            ops.push(Op::Copy(path.strip_prefix("/")?.to_path_buf()));
        }
    }

    Ok(())
}
