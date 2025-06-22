use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::walking_diff::tree::{RemoteTree, Tree};

#[derive(Debug)]
pub enum Op {
    Copy(PathBuf),
    CreateDir(PathBuf),
    Remove(PathBuf),
}

pub fn diff(local: &Tree, remote: &RemoteTree) -> Vec<Op> {
    println!("{:#?}", local.nodes[0]);
    println!("{:#?}", remote.nodes[0]);

    let mut ops = vec![];

    let matches = prune_pass(local, remote, &mut ops);
    creation_pass(local, remote, &mut ops);

    ops
}

fn prune_pass(
    local: &Tree,
    remote: &RemoteTree,
    ops: &mut Vec<Op>,
) -> Vec<(PathBuf, usize, usize)> {
    let mut matched = Vec::new();

    fn walk(
        local: &Tree,
        remote: &RemoteTree,
        local_idx: usize,
        remote_idx: usize,
        path: &Path,
        ops: &mut Vec<Op>,
        matched: &mut Vec<(PathBuf, usize, usize)>,
    ) {
        // Record this node as unchanged
        matched.push((path.to_path_buf(), local_idx, remote_idx));

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
                    &child_path,
                    ops,
                    matched,
                );
            } else {
                // Remote-only: remove subtree
                ops.push(Op::Remove(child_path.to_path_buf()));
            }
        }
    }

    let root = PathBuf::from("/");
    walk(local, remote, 0, 0, &root, ops, &mut matched);
    matched
}

/// Emits operations to create a complete subtree rooted at `local_idx` and `path`.
fn emit_create_subtree(tree: &Tree, local_idx: usize, path: &PathBuf, ops: &mut Vec<Op>) {
    if tree.is_file(local_idx) {
        // File
        ops.push(Op::Copy(path.clone()));
    } else {
        // Directory
        if path != &PathBuf::from("/") {
            ops.push(Op::CreateDir(path.clone()));
        }
        for (name, &child_idx) in &tree.nodes[local_idx].children {
            let mut child_path = path.clone();
            child_path.push(name.as_ref());
            emit_create_subtree(tree, child_idx, &child_path, ops);
        }
    }
}

fn creation_pass(local: &Tree, remote: &RemoteTree, ops: &mut Vec<Op>) {
    fn walk(
        local: &Tree,
        remote: &RemoteTree,
        local_idx: usize,
        remote_idx_opt: Option<usize>,
        path: &PathBuf,
        ops: &mut Vec<Op>,
    ) {
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
                    );
                } else {
                    // Local-only: emit subtree
                    emit_create_subtree(local, l_child_idx, &child_path, ops);
                }
            }
        } else {
            // Remote missing: create entire subtree
            emit_create_subtree(local, local_idx, path, ops);
        }
    }

    let root = PathBuf::from("/");
    walk(local, remote, 0, Some(0), &root, ops);
}

/*
#[derive(Debug)]
pub struct Diff {
    pub update_create_leafs: Vec<OsString>,
    pub create_dirs

}

impl Diff{
    pub fn new() -> Self {
        Self {
            directories_to_create: Vec::new(),

            files_to_copy: Vec::new(),
        }
    }
}


impl Tree {

    /// Diff this tree (local) against another tree (remote)
    /// Returns directories to create and files to copy to
    /// make the remote tree the same as this tree
    ///
    /// MAKES FS OPERATIONS!
    pub async fn diff(&self, remote: &RemoteTree) -> Result<Diff> {
        let mut result = Diff::new();

        // Start comparison from root
        self.diff_recursive(
            0,                  // local root index
            Some(0),            // remote root index (Some if exists)
            "/", // current path
            &remote,
            &mut result,
        );

        // Sort directories by depth (shallow to deep)
        result
            .directories_to_create
            .sort_by_key(|path| path.components().count());

        result
    }

    fn diff_recursive(
        &self,
        local_idx: usize,
        remote_idx: Option<usize>,
        current_path: PathBuf,
        remote_tree: &FileTree,
        result: &mut DiffResult,
    ) {
        let local_node = &self.nodes[local_idx];

        // Iterate through all children in the local tree
        for (child_name, &local_child_idx) in &local_node.children {
            let child_path = if current_path.as_os_str() == "/" {
                PathBuf::from("/").join(child_name)
            } else {
                current_path.join(child_name)
            };

            let local_child = &self.nodes[local_child_idx];

            // Check if this child exists in the remote tree
            let remote_child_idx = remote_idx
                .and_then(|ridx| remote_tree.nodes[ridx].children.get(child_name))
                .copied();

            match (local_child.size.is_none(), remote_child_idx) {
                // Local directory, remote directory exists
                (true, Some(remote_child_idx)) => {
                    // Both are directories, recurse into them
                    self.diff_recursive(
                        local_child_idx,
                        Some(remote_child_idx),
                        child_path,
                        remote_tree,
                        result,
                    );
                }

                // Local directory, remote doesn't exist
                (true, None) => {
                    // Directory doesn't exist remotely, need to create it
                    result.directories_to_create.push(child_path.clone());

                    // Add all files in this directory subtree to copy list
                    self.add_all_files_in_subtree(local_child_idx, child_path, result);
                }

                // Local file, remote file exists
                (false, Some(remote_child_idx)) => {
                    let remote_child = &remote_tree.nodes[remote_child_idx];

                    // Compare file metadata
                    let should_copy = match (&local_child.size, &remote_child.size) {
                        (Some(local_size), Some(remote_size)) => {
                            if local_size != remote_size {
                                return true;
                            }

                            // Files are same size, but they could have diff content. MD5 TIME!
                            let local = File::open(local_child)?;
                            let local_md5 = = md5::compute(local);

                        }
                        _ => true, // Metadata mismatch, copy the file
                    };

                    if should_copy {
                        result.files_to_copy.push(child_path);
                    }
                }

                // Local file, remote doesn't exist
                (false, None) => {
                    // File doesn't exist remotely, need to copy it
                    result.files_to_copy.push(child_path);
                }
            }
        }
    }

    /// Add all files in a subtree to the copy list (used when entire directory is missing)
    fn add_all_files_in_subtree(
        &self,
        node_idx: usize,
        base_path: PathBuf,
        result: &mut DiffResult,
    ) {
        let node = &self.nodes[node_idx];

        for (child_name, &child_idx) in &node.children {
            let child_path = base_path.join(child_name);
            let child_node = &self.nodes[child_idx];

            if child_node.size.is_some() {
                result.files_to_copy.push(child_path);
            } else {
                // It's a directory, create it and recurse
                result.directories_to_create.push(child_path.clone());
                self.add_all_files_in_subtree(child_idx, child_path, result);
            }
        }
    }
}*/

/*use anyhow::Result;
use flipper_rpc::{
    fs::FsReadDir, rpc::res::ReadDirItem, transport::serial::rpc::SerialRpcTransport,
};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::walking_diff::fs::{FileInfo, FsNode};

pub fn diff_tree_recursive<'a, F>(
    cli: &mut SerialRpcTransport,
    local_node: &FsNode,
    transform_fn: &F,
    paths_to_copy: &mut Vec<(&'a Path, PathBuf)>,
    original_paths: &'a [&'a Path],
    failed_remote_paths: &mut std::collections::HashSet<PathBuf>,
) -> Result<()>
where
    F: Fn(&Path) -> PathBuf,
{
    // Skip the root node if it's empty
    if local_node.path.as_os_str().is_empty() {
        for child in local_node.children.values() {
            diff_tree_recursive(
                cli,
                child,
                transform_fn,
                paths_to_copy,
                original_paths,
                failed_remote_paths,
            )?;
        }
        return Ok(());
    }

    // If this is a file, we'll handle it when processing its parent directory
    if local_node.is_file {
        return Ok(());
    }

    // This is a directory - check if any parent path has already failed
    let local_dir = &local_node.path;
    let remote_dir = transform_fn(local_dir);

    // Fast fail: check if any parent directory has already failed
    let should_skip = failed_remote_paths
        .iter()
        .any(|failed_path| remote_dir.starts_with(failed_path));

    if should_skip {
        // Parent directory failed, mark all files in this subtree for copying
        mark_subtree_for_copy(local_node, transform_fn, paths_to_copy, original_paths);
        return Ok(());
    }

    // Build local file info map
    let mut local_files = HashMap::new();

    // Collect files from this directory level only
    for (name, child) in &local_node.children {
        if child.is_file {
            if let Ok(file_info) = get_local_file_info(&child.path) {
                local_files.insert(name.clone(), file_info);
            }
        }
    }

    // Read remote directory with better error handling
    let (remote_files, remote_dir_exists) = match cli.fs_read_dir(&remote_dir) {
        Ok(items) => {
            let mut files = HashMap::new();
            for item in items {
                if let ReadDirItem::File(name, size, hash) = item {
                    files.insert(name, FileInfo { size, hash });
                }
            }
            (files, true)
        }
        Err(e) => {
            // Directory doesn't exist or is inaccessible - mark as failed
            eprintln!(
                "Remote directory {} not accessible: {:?}",
                remote_dir.display(),
                e
            );
            failed_remote_paths.insert(remote_dir.clone());

            // Mark all files in this subtree for copying
            mark_subtree_for_copy(local_node, transform_fn, paths_to_copy, original_paths);
            return Ok(());
        }
    };

    // Compare files in this directory
    for (name, local_info) in &local_files {
        let local_file_path = local_dir.join(name);
        let remote_file_path = remote_dir.join(name);

        let needs_copy = match remote_files.get(name) {
            Some(remote_info) => {
                // File exists remotely, compare size and hash
                local_info.size != remote_info.size || local_info.hash != remote_info.hash
            }
            None => {
                // File doesn't exist in remote directory
                true
            }
        };

        if needs_copy {
            // Find the original path from our input list
            if let Some(&original_path) = original_paths
                .iter()
                .find(|&&p| p == local_file_path.as_path())
            {
                paths_to_copy.push((original_path, remote_file_path));
            }
        }
    }

    // Recursively process subdirectories
    for child in local_node.children.values() {
        if !child.is_file {
            diff_tree_recursive(
                cli,
                child,
                transform_fn,
                paths_to_copy,
                original_paths,
                failed_remote_paths,
            )?;
        }
    }

    Ok(())
}

fn get_local_file_info(path: &Path) -> Result<FileInfo> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len() as u32;
    let hash = calculate_md5_hash(path)?;

    Ok(FileInfo { size, hash })
}

fn calculate_md5_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let digest = md5::compute(&buffer);
    Ok(format!("{:x}", digest))
}

fn mark_subtree_for_copy<'a, F>(
    node: &FsNode,
    transform_fn: &F,
    paths_to_copy: &mut Vec<(&'a Path, PathBuf)>,
    original_paths: &'a [&'a Path],
) where
    F: Fn(&Path) -> PathBuf,
{
    // Mark all files in this subtree for copying
    for (_name, child) in &node.children {
        if child.is_file {
            let local_file_path = &child.path;
            let remote_file_path = transform_fn(local_file_path);

            // Find the original path from our input list
            if let Some(&original_path) = original_paths
                .iter()
                .find(|&&p| p == local_file_path.as_path())
            {
                paths_to_copy.push((original_path, remote_file_path));
            }
        } else {
            // Recursively mark subdirectories
            mark_subtree_for_copy(child, transform_fn, paths_to_copy, original_paths);
        }
    }
}*/
