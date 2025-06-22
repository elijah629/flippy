use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};

use anyhow::bail;
use flipper_rpc::{
    fs::{FsRead, FsWrite},
    transport::serial::{list_flipper_ports, rpc::SerialRpcTransport},
};
use gix::{
    ObjectId,
    bstr::ByteSlice,
    diff::{Options, index::Action},
    index::State,
    objs::find::Never,
    open,
    pathspec::Search,
};
use tokio::fs;
use tracing::info;

use crate::{
    types::{
        flip::Flip,
        mapping::{Mapping, MappingInfo},
    },
    walking_diff::{
        self,
        diff::Op,
        tree::{RemoteNode, RemoteTree},
    },
};

pub async fn run(flip: Flip) -> anyhow::Result<()> {
    let ports = list_flipper_ports()?;

    /*let port = if ports.len() > 1 {
        let items: Vec<&str> = ports.iter().map(|x| x.device_name.as_str()).collect();
        let selection = select("Which device is the target Flipper Zero?")
            .items(&items)
            .interact()?;

        &ports[selection].port_name // .0 is the port_name
    } else {
        &ports[0].port_name
    };*/

    let port = &ports[0].port_name;

    let mut cli = SerialRpcTransport::new(port)?;

    //info!("Selected flipper on port {}", port);

    // TODO: Implement a walking diff algorithim, Needs to list every file already on the flipper
    // with its MD5 hash (use flipper_rpc) then, after comparing with the files we need to send and
    // their MD5 hashes (should make a hashfile to store them, although inexpensive these can stack
    // up quick). Then only copy the files that have been updated.
    //
    // TODO: (Cont...) Copy all files that dont exist. And create folders that don't exist.
    // ( negative diffed from SRC -> Flipper )
    //
    // TODO: Implement SD card writing. Option to use the SD card directly instead of sending files
    // over RPC for more speed.
    let path = flip.source_path;

    for (name, repo) in flip.repositories.iter() {
        let url = gix::url::parse(repo.url.as_str().into())?;
        let path = path.join("store").join(repo.uuid.to_string());

        if !fs::try_exists(&path).await? {
            bail!(
                "repository `{name}` at `{url}` does not exist but is in store\n\t\tplease run `flippy store fetch` to download all repositories"
            );
        }

        let mappings = &repo.mappings;

        let repo = open(&path)?;

        /*
        // Retrieve the specified commit
        let commit = repo.find_commit(ObjectId::from_hex(b"a74e5bfe0122394cf064eb8ed6bbc3770b7d8ad3")?)?;

        // Retrieve the current HEAD commit
        let head_commit = repo.head()?.state;

        // Obtain the trees of both commits
        let commit_tree = commit.tree()?;
        let head_tree = head_commit.tree()?;

        // Compute the diff between the two trees
        let diff = commit_tree.diff(&head_tree, FileMode::NORMAL)?;*/

        // Iterate over the diffs and print the changed files
        /*for delta in diff.deltas() {
            println!("Changed file: {}", delta.new_file().path().display());
        } */       //let index = repo.try_index()?.expect("index could not be opened");
        //

        let head = repo.head_commit()?;
        println!("{:?}", head.id.to_string());
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
                    // be updated here using gix's diff

                    let remote_hash: [u8; 20] = remote_hash.as_ref().try_into()?;

                    let remote_commit = repo.find_commit(remote_hash)?;
                    println!("{:?}", remote_commit.id.to_string());
                    let remote_tree = remote_commit.tree()?;

                    let local_tree = repo.head_tree()?;
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

                    for change in repo.diff_tree_to_tree(&remote_tree, &local_tree, None)? {
                        let r#type = match &change {
                            gix::diff::tree_with_rewrites::Change::Addition {
                                location,
                                relation,
                                entry_mode,
                                id,
                            } => "A",
                            gix::diff::tree_with_rewrites::Change::Deletion {
                                location,
                                relation,
                                entry_mode,
                                id,
                            } => "D",
                            gix::diff::tree_with_rewrites::Change::Modification {
                                location,
                                previous_entry_mode,
                                previous_id,
                                entry_mode,
                                id,
                            } => "M",
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
                            } => "R",
                        };
                        println!("{type} {}", change.location());
                    }
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
                Err(flipper_rpc::error::Error::Rpc(
                    flipper_rpc::rpc::error::Error::StorageError(
                        flipper_rpc::rpc::error::StorageError::NotFound,
                    ),
                )) => {
                    // Oh no! The flipper has not been initialized. We must manually copy using
                    // a tree diff since the .flippy_commit_hash file could have been deleted. If
                    // it was deleted the upload will take a fuckton longer if there is a lot of
                    // files. This uses my custom walking_diff algorithim.

                    cli.fs_write(remote_hash_path, head.id.as_slice())?;

                    todo!()
                }
                Err(e) => return Err(e.into()), // Generic error
            }
        }
        /*
        if let Err(e) = hash {
            println!("{}", e); // ERROR_STORAGE_NOT_EXIST

            cli.fs_write(flippy, head.id.to_string())?;

            // Fallback to a custom fast-diff (NO I DID NOT DISCOVER GIX DIFF AFTER MAKING
            // THIS)
            //let tree =
            //  walking_diff::tree::RemoteTree::from_remote(&mut cli, &destination, ignore);

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
    }

    Ok(())
}

/// Counts the number of reachable nodes (files + directories) from the root of the RemoteTree,
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
}
