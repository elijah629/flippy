use crate::{
    Flip,
    commands::upload::{Commit, Path, bail, info, open, pathspec::pathspec_from_pattern},
    git::diff::diff_from_head,
    types::{
        mapping::MappingInfo,
        remote_sync_file::{Repo, SyncFile},
    },
    walking_diff::{self, diff::Op},
};
use anyhow::{Context, Result};
use flipper_rpc::{
    fs::{FsReadDir, helpers::os_str_to_str},
    transport::serial::rpc::SerialRpcTransport,
};
use fxhash::{FxBuildHasher, FxHashMap};
use gix::{Pathspec, bstr::ByteSlice};
use std::ffi::OsString;
use std::path::PathBuf;
use tokio::fs;

pub async fn diff_all_repositories(
    flip: &Flip,
    cli: &mut SerialRpcTransport,
    operations: &mut Vec<Op>,
    sync_file: SyncFile,
    updated_sync_file: &mut SyncFile,
) -> Result<()> {
    for (name, repo) in &flip.repositories {
        let url = gix::url::parse(repo.url.as_str().into())?;
        let uuid = repo.uuid;
        let path = flip.source_path.join("store").join(uuid.to_string());

        if !fs::try_exists(&path).await? {
            bail!(
                "repository `{name}` at `{url}` does not exist but is in store\n\t\tplease run `flippy store fetch` to download all repositories"
            );
        }

        let mappings = repo.mappings.iter();
        let repo = open(&path)?;

        let head_hash = repo.head_commit()?.id;
        let head_hash = head_hash.as_slice();

        let remote_hash = sync_file.find_hash(&uuid);

        operations.push(Op::Repo(path.clone()));

        match remote_hash {
            Some(remote_hash) => {
                info!("Using `git diff` with the state from the sync file");

                let remote_commit = repo.find_commit(*remote_hash)?;

                for mapping in mappings {
                    let MappingInfo {
                        patterns: p,
                        destination,
                        ignore: _,
                    } = mapping.info();

                    let (mut spec, _) = pathspec_from_pattern(&repo, p.patterns())?;

                    let search = spec.search();

                    let lcd_path = search
                        .longest_common_directory()
                        .context("longest_common_directory was None")?;

                    let lcd = os_str_to_str(lcd_path.as_os_str())?;

                    // Marker for this mapping
                    operations.push(Op::Mapping(lcd.to_string(), destination));

                    // Now generate the git-based adds/removes under this mapping
                    git_diff(remote_commit.clone(), operations, &mut spec)
                        .context("failed to run git_diff for mapping")?;
                }
            }
            None => {
                info!(
                    "Using the slower walking_diff method, this will only happen once for an initial clone."
                );

                for mapping in mappings {
                    let MappingInfo {
                        patterns: p,
                        destination,
                        ignore,
                    } = mapping.info();

                    let (
                        //
                        mut spec,
                        local_state,
                    ) = pathspec_from_pattern(&repo, p.patterns())?;

                    let search = spec.search();

                    // Remove the folder path from the repository, it will be readded when we operate on
                    // it
                    let lcd_path = search
                        .longest_common_directory()
                        .context("longest_common_directory was None")?;

                    let lcd = os_str_to_str(lcd_path.as_os_str())?;

                    operations.push(Op::Mapping(lcd.to_string(), destination));

                    let local_root = path.join(&lcd_path);
                    let removal_length = lcd.len() + 1;

                    let paths = spec
                        .index_entries_with_paths(&local_state).context("Index was empty, no files to change. You may ignore this if your pathspecs did not match anything")?.map(|(str, entry)| {
                            let str = &str[removal_length..];
                            let str = str.to_os_str().context("Path was not UTF-8").unwrap();
                            (Path::new(str), entry.stat.size)
                        })
                        .collect::<Vec<_>>();

                    walking_diff(
                        //
                        cli,
                        &paths,
                        &local_root,
                        destination,
                        ignore,
                        operations,
                    )?;
                }
            }
        }

        updated_sync_file.repositories.push(Repo {
            uuid: *uuid.as_bytes(),
            hash: head_hash.try_into()?,
        });
    }

    Ok(())
}

fn git_diff(remote_commit: Commit<'_>, ops: &mut Vec<Op>, search: &mut Pathspec) -> Result<()> {
    ops.extend(
        diff_from_head(remote_commit)?
            .iter()
            .filter_map(|change| match change {
                gix::diff::tree_with_rewrites::Change::Addition { location, .. } => {
                    let location = location.to_str().unwrap();
                    if !search.is_included(location, Some(false)) {
                        return None;
                    }
                    Some(Op::Copy(PathBuf::from(location)))
                }
                gix::diff::tree_with_rewrites::Change::Deletion { location, .. } => {
                    let location = location.to_str().unwrap();
                    if !search.is_included(location, Some(false)) {
                        return None;
                    }
                    Some(Op::Remove(PathBuf::from(location)))
                }
                gix::diff::tree_with_rewrites::Change::Modification { location, .. } => {
                    let location = location.to_str().unwrap();
                    if !search.is_included(location, Some(false)) {
                        return None;
                    }
                    Some(Op::Copy(PathBuf::from(location)))
                }
                _ => unreachable!("rewrites are disabled"),
            }),
    );

    Ok(())
}

fn walking_diff<P: AsRef<Path> + Sync>(
    cli: &mut SerialRpcTransport,
    local_paths: &[(P, u32)],
    local_root: impl AsRef<Path>,
    remote_root: impl AsRef<Path>,
    remote_ignore: &'static [&'static str],
    ops: &mut Vec<Op>,
) -> Result<()> {
    info!("Creating local tree");
    let local_tree = walking_diff::tree::Tree::from_path_and_sizes(local_paths);
    info!("Creating remote tree");
    let remote_tree =
        walking_diff::tree::RemoteTree::from_remote(cli, &remote_root, remote_ignore)?;

    let mut remote_hashes: FxHashMap<usize, [u8; 16]> =
        FxHashMap::with_hasher(FxBuildHasher::new());

    let local_root = local_root.as_ref();
    let remote_root = remote_root.as_ref();

    info!("Diffing");
    walking_diff::diff::diff(
        &local_tree,
        &remote_tree,
        ops,
        // TODO: Cache MD5's for directories. Use read_dir instead, HOW: Pass parent index into
        // this function and build a hashmap here.
        |common_file_path, local_node_size, remote_node_idx, remote_node_parent| {
            let remote_node = &remote_tree.nodes[remote_node_idx];

            Ok(local_node_size != remote_node.size || {
                let common_file_path = common_file_path.strip_prefix("/")?;
                let local = std::fs::read(local_root.join(common_file_path))?;
                let local_hash = md5::compute(local);
                let local_hash = hex::encode(*local_hash);

                let remote_hash = match remote_hashes.get(&remote_node_idx) {
                    Some(hash) => hash,
                    None => {
                        let parent_dir = remote_root.join(common_file_path);
                        let parent_dir = parent_dir.parent().unwrap();

                        let hashes =
                            cli.fs_read_dir(parent_dir, true)?
                                .filter_map(|item| match item {
                                    flipper_rpc::rpc::res::ReadDirItem::Dir(_) => None,
                                    flipper_rpc::rpc::res::ReadDirItem::File(name, _size, md5) => {
                                        Some((name, md5.unwrap()))
                                    }
                                });

                        for (name, hash) in hashes {
                            let remote_node_parent_child = remote_tree
                                .find_child_by_name(remote_node_parent, &OsString::from(name))
                                .unwrap();

                            let mut md5 = [0u8; 16];
                            hex::decode_to_slice(hash, &mut md5)?;

                            remote_hashes.insert(*remote_node_parent_child, md5);
                        }

                        remote_hashes.get(&remote_node_idx).unwrap()
                    }
                };

                let remote_hash = hex::encode(remote_hash);

                // TODO: Cache results into the store somehow
                let diff = remote_hash != local_hash;
                info!(
                    local_hash,
                    remote_hash,
                    "diff {}",
                    if diff { '❌' } else { '✅' }
                );
                diff
            })
        },
    )?;

    Ok(())
}
