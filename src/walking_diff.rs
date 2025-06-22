pub mod diff;
pub mod tree;

/*use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

mod fs;
use fs::*;

use anyhow::{Result, anyhow, bail};
use flipper_rpc::{
    fs::FsReadDir, rpc::res::ReadDirItem, transport::serial::rpc::SerialRpcTransport,
};

/// Filters a list of local paths by comparing them to remote paths.
///
/// First compares size, if the size is different remove it. If it is the same, compare MD5 hashes.
/// If it is different remove it. If it is same, keep it.
///
/// Reads directories as a whole on the flipper to speed it up. Uses cli.fs_read_dir
///
/// # Returns filtered list
pub fn walking_diff<'a>(
    cli: &mut SerialRpcTransport,
    paths: impl ExactSizeIterator<Item = (&'a Path, &'a Path)>,
) -> Result<impl Iterator<Item = (&'a Path, &'a Path)>> {
    let mut by_dir: HashMap<PathBuf, Vec<(&Path, &Path)>> = HashMap::new();
    for (local, remote) in paths {
        let dir = remote
            .parent()
            .ok_or_else(|| anyhow!("remote has no parent: {:?}", remote))?
            .to_path_buf();
        by_dir.entry(dir).or_default().push((local, remote));
    }

    // 2. For each remote dir, get all File entries (with MD5)
    let mut meta_map: HashMap<PathBuf, HashMap<String, (u32, String)>> = HashMap::new();
    for (dir, list) in &by_dir {
        let entries = cli.fs_read_dir(dir)?;
        let mut map = HashMap::new();
        for file in entries {
            if let ReadDirItem::File(name, size, md5) = file {
                map.insert(name, (size, md5));
            }
        }
        meta_map.insert(dir.clone(), map);
    }

    // 3. Filter original pairs by size and MD5 match
    let filtered = by_dir.into_iter().flat_map(move |(dir, list)| {
        let lookup = &meta_map[&dir];
        list.into_iter().filter_map(move |(local, remote)| {
            let filename = remote.file_name()?.to_str()?.to_string();
            let (remote_size, remote_md5) = lookup.get(&filename)?;
            // size check
            if std::fs::metadata(local).map_or(false, |m| m.len() == *remote_size) {
                // md5 check
                if let Ok(local_md5) = compute_local_md5(local) {
                    if local_md5.as_ref() == remote_md5 {
                        return Some((local, remote));
                    }
                }
            }
            None
        })
    });

    Ok(filtered)
}*/

//mod diff;

//use diff::diff_tree_recursive;
//use fs::FsNode;

/*fn walking_diff<'a, F>(
    cli: &mut SerialRpcTransport,
    paths: impl ExactSizeIterator<Item = &'a Path>,
    transform_fn: F,
) -> Result<impl Iterator<Item = (&'a Path, PathBuf)>>
where
    F: Fn(&Path) -> PathBuf,
{
    let path_list: Vec<_> = paths.collect();

    // Build the local filesystem tree from flat paths
    let local_tree = FsNode::from_paths(&path_list);

    // Track which remote paths have failed to avoid redundant checks
    let mut failed_remote_paths = std::collections::HashSet::new();

    // Collect all paths that need copying
    let mut paths_to_copy = Vec::new();

    // Traverse the tree and diff each directory
    diff_tree_recursive(
        cli,
        &local_tree,
        &transform_fn,
        &mut paths_to_copy,
        &path_list,
        &mut failed_remote_paths,
    )?;

    Ok(paths_to_copy.into_iter())
}


fn wlkdff(cli: &mut SerialRpcTransport, local_paths: )*/
