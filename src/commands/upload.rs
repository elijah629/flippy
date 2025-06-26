use crate::flipper::pick_cli;
use crate::{
    commands::upload::diff::diff_all_repositories, types::remote_sync_file::SYNC_FILE_PATH,
};
use crate::{
    types::{flip::Flip, remote_sync_file::SyncFile},
    walking_diff::diff::Op,
};
use anyhow::{Result, bail};
use cliclack::confirm;
use flipper_rpc::{
    fs::{FsCreateDir, FsRead, FsRemove, FsWrite},
    transport::serial::rpc::SerialRpcTransport,
};
use gix::{Commit, open};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{info, warn};

mod diff;
mod pathspec;

pub async fn run(flip: Flip, force_walkdir: bool) -> Result<()> {
    if force_walkdir {
        unimplemented!(
            "--force-walkdir is not implemented yet. Pleasse just delete {SYNC_FILE_PATH} manually"
        );
    }

    let mut cli = pick_cli()?;

    // TODO: Implement SD card writing. Option to take out the SD card and write to it directly (if the host has a SD reader), instead of sending files through RPC. this will improve speed greatly for those who can

    let sync_file = cli
        .fs_read(SYNC_FILE_PATH)
        .map_err(Into::into)
        .and_then(SyncFile::deserialize);

    let mut operations: Vec<Op> = vec![];
    let mut updated_sync_file = SyncFile {
        repositories: Vec::with_capacity(flip.repositories.len()),
    };

    match sync_file {
        Ok(sync_file) => {
            // Yes! The syncfile exists already. We must check if each REPO exists inside of it,
            // but we have *some* data.
            //
            // Rebuild the syncfile after every iteration, then write it at the end
            diff_all_repositories(
                &flip,
                &mut cli,
                &mut operations,
                sync_file,
                &mut updated_sync_file,
            )
            .await?;
        }
        Err(e)
            if matches!(
                e.downcast_ref::<flipper_rpc::error::Error>(),
                Some(flipper_rpc::error::Error::Rpc(
                    flipper_rpc::rpc::error::Error::StorageError(
                        flipper_rpc::rpc::error::StorageError::NotFound
                    )
                ))
            ) =>
        {
            // Oh no! The flipper has not been initialized. We must manually copy using
            // a tree diff FOR EVERY REPOSITORY since the .flippy_do_not_remove syncfile
            // could have been deleted. If  it was deleted the upload will take a fuckton
            // longer if there is a lot of files. This uses my custom walking_diff algorithim.

            warn!("Sync file at '{SYNC_FILE_PATH}' does not exist.");
            warn!(
                "Ignore the previous message if this your first time running this command on this flipper."
            );
            warn!(
                "If it is not your first time, please take care in keeping that file and make a backup of it. The sync file holds important information to drastically improve transfer speeds and I/O calls. Over time many I/O calls will wear out your SD card."
            );

            info!("Creating blank sync file, re-run the command to compute a diff");
        }
        Err(e) => return Err(e),
    }

    let (mut copy, mut dir, mut remove) = (0usize, 0usize, 0usize);

    for op in &operations {
        match op {
            Op::Copy(..) => copy += 1,
            Op::CreateDir(..) => dir += 1,
            Op::Remove(..) => remove += 1,
            _ => (),
        }
    }

    let count = copy + dir + remove;

    if count == 0 {
        info!("All good, no operations to do.");
        cli.fs_write(SYNC_FILE_PATH, updated_sync_file.serialize(), None)?;
        return Ok(());
    }

    let confirm = confirm(format!(
        "Perform {count} operation(s)? (cp {copy}, mkdir {dir}, rm {remove})",
    ))
    .interact()?;

    if !confirm {
        bail!("Aborted");
    }

    info!("Doing those aforementioned operations");

    let mut repo = &PathBuf::new();
    let mut mapping_root_local = PathBuf::new();
    let mut mapping_root_remote = PathBuf::new();

    for op in &operations {
        match op {
            Op::Repo(path_buf) => {
                repo = path_buf;
            }
            Op::Mapping(local, remote) => {
                mapping_root_local = PathBuf::from(local);
                mapping_root_remote = PathBuf::from(remote);
            }
            Op::Copy(path_buf) => {
                let from = repo.join(&mapping_root_local).join(path_buf);
                let to = mapping_root_remote.join(path_buf);
                info!("copy   {from:?} -> {to:?}");

                let from = fs::read(from).await?;
                cli.fs_write(to, from, None)?;
            }
            Op::CreateDir(path_buf) => {
                let to = mapping_root_remote.join(path_buf);
                info!("mkdir  {to:?}");

                cli.fs_create_dir(to)?;
            }
            Op::Remove(path_buf) => {
                let to = mapping_root_remote.join(path_buf);

                info!("remove {to:?}");
                cli.fs_remove(to, true)?;
            }
        }
    }

    // If you select NO on the confirm, running the command again will make itself think that it
    // ran the last time, desyncing the commit hash
    // Only update if it didnt fail (likely in beta)
    cli.fs_write(SYNC_FILE_PATH, updated_sync_file.serialize(), None)?;

    Ok(())
}

/*
*
* /*for (name, repo) in flip.repositories {
                let url = gix::url::parse(repo.url.as_str().into())?;
                let uuid = repo.uuid;
                let path = path.join("store").join(uuid.to_string());

                if !fs::try_exists(&path).await? {
                    bail!(
                        "repository `{name}` at `{url}` does not exist but is in store\n\t\tplease run `flippy store fetch` to download all repositories"
                    );
                }

                let mappings = &repo.mappings;
                let repo = open(&path)?;
                let local_state = repo.index()?;

                let spec = repo.pathspec(
                    false,
                    mappings.iter().filter_map(|x| {
                        let info = x.info();
                        info.pattern.to_str()
                    }),
                    false,
                    &local_state,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                )?;

                let head_hash = repo.head_commit()?.id;
                let head_hash = head_hash.as_slice();

                info!("walking_diff_forced");

                updated_sync_file.repositories.push(Repo {
                    uuid: *uuid.as_bytes(),
                    hash: head_hash.try_into()?,
                });
            }*/

for (name, repo) in flip.repositories.iter() {
    let head = repo.head_commit()?;

    let mut operations = vec![];

    for mapping in mappings {
        let MappingInfo {
            pattern,
            destination,
            ignore,
        } = mapping.info();

        let remote_hash_path = Path::new(destination).join(".flippy_commit_hash");
        let remote_hash = cli.fs_read(&remote_hash_path);

        match remote_hash {
            Ok(remote_hash) => {
                // Yay! The flipper has already been initialized with the repo and is going to
                // be updated here using gix's diff, which is by far much faster (and optimized)
                // than my thoopid diff

                let remote_hash: [u8; 20] = remote_hash.as_ref().try_into()?;

                let diff = diff_from_head(remote_commit)?;

                operations.extend(diff.iter().map(|change| match change {
                    gix::diff::tree_with_rewrites::Change::Addition {
                        location,
                        relation,
                        entry_mode,
                        id,
                    } => {
                        println!("A {location} {relation:?} {entry_mode:?} {id}");
                    }
                    gix::diff::tree_with_rewrites::Change::Deletion {
                        location,
                        relation,
                        entry_mode,
                        id,
                    } => {
                        println!("D {location} {relation:?} {entry_mode:?} {id}");
                    }
                    gix::diff::tree_with_rewrites::Change::Modification {
                        location,
                        previous_entry_mode,
                        previous_id,
                        entry_mode,
                        id,
                    } => {
                        println!("M {location} {previous_entry_mode:?} {previous_id:?} {entry_mode:?} {id}");
                    }
                    gix::diff::tree_with_rewrites::Change::Rewrite {
                        source_location,
                        source_entry_mode,
                        source_relation,
                        source_id,
                        diff,
                        entry_mode,
                        id,
                        location,
                        relation,
                        copy,
                    } => {
                        println!("R {source_location} {source_entry_mode:?} {source_relation:?} {source_id} {diff:?} {entry_mode:?} {id} {location} {relation:?} {copy}")
                    },
                }));

                /*
                let pattern = pattern.to_string_lossy();
                let pattern = pattern.as_ref();

                let (search, att) = repo
                    .pathspec(
                        false,
                        [pattern],
                        false,
                        &local_state,
                        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
                    )?
                    .into_parts();
                let mut spec = search;*/

                /*
                let diff = gix::diff::index(
                    &remote_state,
                    &local_state,
                    |action| -> anyhow::Result<Action> {
                        println!("{action:?}");
                        anyhow::Ok::<Action>(gix::diff::index::Action::Continue)
                    },
                    None::<gix::diff::index::RewriteOptions<'_, Never>>,
                    &mut spec,
                    &mut |_str, _case, _bo, _outcome| true,
                )?;

                println!("{:?}", diff);*/
                //let remote_state = State::from_tree(tree, , validate)?
                //                    let remote_state =  repo.commit
            }

            Err(e) => return Err(e.into()), // Generic error
        }
    }*/
/*
if let Err(e) = hash {
    println!("{}", e); // ERROR_STORAGE_NOT_EXIST

    cli.fs_write(flippy, head.id.to_string())?;

    // Fallback to a custom fast-diff (NO I DID NOT DISCOVER GIX DIFF AFTER MAKING
    // THIS)
    //let tree =
    //  walking_diff::tree::remotetree::from_remote(&mut cli, &destination, ignore);

    continue;
}

let hash = hash.unwrap();
let hash = hash.as_ref();
//            let hash = ObjectId::from_str(hash)?;
let commit = repo.find_commit(hash);

println!("{:?}", hash);

//let commit = ObjectId::

let pattern = pattern.to_string_lossy();
let pattern = pattern.as_ref();

/*let mut spec = repo
    .pathspec(
        false,
        [pattern],
        false,
        &index,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?
    .search();

gix::diff::index(
    remote_index,
    &index,
    |change| {
        println!("{change:?}");

        Ok(gix::diff::index::Action::Continue)
    },
    None,
    &mut spec,
    &mut |str, case, bo, outcome| false,
);*/*/
/*
for mapping in mappings {
    let MappingInfo {
        pattern,
        destination,
        ignore,
    } = mapping.info();

    let pattern = pattern.to_string_lossy();
    let pattern = pattern.as_ref();

    let mut spec = repo.pathspec(
        false,
        [pattern],
        false,
        &index,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?;

    let longest_common_directory = spec.search().longest_common_directory();

    // Remove the folder path from the repository, it will be readded when we operate on
    // it
    let removal_length = longest_common_directory
        .map(|x| x.to_string_lossy().len())
        .unwrap_or(0)
        + 1;

    if let Some(entries) = spec.index_entries_with_paths(&index) {
        let entries = entries
            .map(|(str, _)| {
                let str = &str[removal_length..];
                let str = str.to_os_str().unwrap();

                Path::new(str)
            })
            .collect::<Vec<_>>();

        /*for entry in entries {
            println!("{}", entry.display());
        }*/

        let local = walking_diff::tree::Tree::from_paths(&entries);

        let mut remote =
            walking_diff::tree::RemoteTree::from_remote(&mut cli, destination, ignore)?;

        let operations = walking_diff::diff::diff(&local, &remote);

        let old = remote.nodes.len();
        println!("{:?}", old);
        apply_ops(&mut remote, &operations);
        println!("{:?} == {:?}", count_reachable(&remote), local.nodes.len());
        //println!("{:?}", diff);

        //println!("{}", arena.nodes.len());
        //let tree = walking_diff::tree::RemoteTree::from_remote(&mut cli, "/ext")?;

        //print_tree(&tree, 0);
    }
}*/

/*
// Counts the number of reachable nodes (files + directories) from the root of the RemoteTree,
/// excluding the root itself. Use this to compare to a local Tree.
use std::collections::HashSet;

pub fn count_reachable(remote: &RemoteTree) -> usize {
    fn walk(tree: &RemoteTree, idx: usize, visited: &mut HashSet<usize>) {
        if !visited.insert(idx) {
            return;
        }
        for &child_idx in tree.nodes[idx].children.values() {
            walk(tree, child_idx, visited);
        }
    }

    let mut visited = HashSet::new();
    walk(remote, 0, &mut visited);
    // Subtract the root node from count
    visited.len()
}

/// Utility: find a node index by path (absolute, starting at root "/").
fn find_node_mut(tree: &mut RemoteTree, path: &Path) -> Option<usize> {
    let mut idx = 0;
    for comp in path.components().skip(1) {
        let name = comp.as_os_str();
        let node = &tree.nodes[idx];
        if let Some(&child_idx) = node.children.get(name) {
            idx = child_idx;
        } else {
            return None;
        }
    }
    Some(idx)
}

/// Removes a child (and its subtree) from a parent, without adjusting the flat nodes Vec.
fn remove_path(tree: &mut RemoteTree, path: &Path) {
    if let Some(parent_path) = path.parent() {
        if let Some(parent_idx) = find_node_mut(tree, parent_path) {
            if let Some(name) = path.file_name() {
                tree.nodes[parent_idx].children.remove(name);
            }
        }
    }
}

/// Creates a directory at `path` in the RemoteTree, cloning structure.
fn create_dir(tree: &mut RemoteTree, path: &Path) {
    if let Some(parent_path) = path.parent() {
        if let Some(parent_idx) = find_node_mut(tree, parent_path) {
            if let Some(name) = path.file_name() {
                tree.add_child_to(RemoteNode::new(name, None), parent_idx);
            }
        }
    }
}

/// Copies a file at `path` into the RemoteTree, using dummy size 0.
fn create_file(tree: &mut RemoteTree, path: &Path) {
    if let Some(parent_path) = path.parent() {
        if let Some(parent_idx) = find_node_mut(tree, parent_path) {
            if let Some(name) = path.file_name() {
                tree.add_child_to(RemoteNode::new(name, Some(0)), parent_idx);
            }
        }
    }
}

/// Simulates running a sequence of operations on a RemoteTree in-place.
pub fn apply_ops(remote: &mut RemoteTree, ops: &[Op]) {
    for op in ops {
        match op {
            Op::Remove(path) => remove_path(remote, path),
            Op::CreateDir(path) => create_dir(remote, path),
            Op::Copy(path) => create_file(remote, path),
        }
    }
}

/// Print the entire tree from `root_idx`.
pub fn print_tree(arena: &RemoteTree, root_idx: usize) {
    // Print the root
    println!("{}", arena.nodes[root_idx].name.to_string_lossy());

    // Collect and sort the children by name
    let mut kids: Vec<_> = arena.nodes[root_idx].children.iter().collect();
    kids.sort_by_key(|entry| entry.0.clone());

    // Recurse
    for (i, &(_name, &child_idx)) in kids.iter().enumerate() {
        let last = i == kids.len() - 1;
        print_tree_node(arena, child_idx, String::new(), last);
    }
}

fn print_tree_node(arena: &RemoteTree, idx: usize, prefix: String, is_last: bool) {
    let name = &arena.nodes[idx].name;
    let connector = if is_last { "└── " } else { "├── " };

    println!("{}{}{}", prefix, connector, name.to_string_lossy());

    // Build next‐level prefix
    let next_prefix = if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    // Collect and sort children
    let mut kids: Vec<_> = arena.nodes[idx].children.iter().collect();
    kids.sort_by_key(|entry| entry.0.clone());

    // Recurse
    for (i, &(_child_name, &child_idx)) in kids.iter().enumerate() {
        let last = i == kids.len() - 1;
        print_tree_node(arena, child_idx, next_prefix.clone(), last);
    }
}*/
