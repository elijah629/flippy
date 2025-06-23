use std::path::PathBuf;

use crate::Flip;
use crate::commands::upload::Commit;
use crate::commands::upload::Path;
use crate::commands::upload::SerialRpcTransport;
use crate::commands::upload::bail;
use crate::commands::upload::info;
use crate::commands::upload::open;
use crate::commands::upload::pathspec::pathspec_from_pattern;
use crate::git::diff::diff_from_head;
use crate::types::mapping::MappingInfo;
use crate::types::remote_sync_file::Repo;
use crate::types::remote_sync_file::SyncFile;
use crate::walking_diff;
use crate::walking_diff::diff::Op;
use anyhow::Result;
use gix::Pathspec;
use gix::bstr::ByteSlice;
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

        operations.push(Op::Repo(path));

        match remote_hash {
            Some(remote_hash) => {
                info!("Using `git diff` with the state from the sync file");

                let remote_commit = repo.find_commit(*remote_hash)?;

                let patterns =
                    mappings.flat_map(|x| x.info().patterns.patterns().collect::<Vec<_>>());

                let (mut spec, _) = pathspec_from_pattern(&repo, patterns)?;

                git_diff(remote_commit, operations, &mut spec)?;
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

                    let (mut spec, local_state) = pathspec_from_pattern(&repo, p.patterns())?;

                    let search = spec.search();

                    // Remove the folder path from the repository, it will be readded when we operate on
                    // it
                    let removal_length = if let Some(longest_common_directory) =
                        search.longest_common_directory()
                    {
                        let longest_common_directory = longest_common_directory.to_str().unwrap();
                        operations.push(Op::Mapping(
                            longest_common_directory.to_string(),
                            destination,
                        ));

                        longest_common_directory.len() + 1
                    } else {
                        operations.push(Op::Mapping("".to_string(), destination));

                        1
                    };

                    let paths = spec
                        .index_entries_with_paths(&local_state)
                        .unwrap()
                        .map(|(str, entry)| {
                            let str = &str[removal_length..];
                            let str = str.to_os_str().unwrap();
                            (Path::new(str), entry.stat.size)
                        })
                        .collect::<Vec<_>>();

                    walking_diff(cli, &paths, destination, ignore, operations)?;
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
                gix::diff::tree_with_rewrites::Change::Addition {
                    location, relation, ..
                } => {
                    let location = location.to_str().unwrap();
                    if !search.is_included(location, Some(false)) {
                        return None;
                    }
                    println!("{:?}", relation);
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
                /*gix::diff::tree_with_rewrites::Change::Rewrite {
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

                },*/
                _ => unreachable!(),
            }),
    );

    Ok(())
}

fn walking_diff<P: AsRef<Path> + Sync>(
    cli: &mut SerialRpcTransport,
    paths: &[(P, u32)],
    root: impl AsRef<Path>,
    ignore: &'static [&'static str],
    ops: &mut Vec<Op>,
) -> Result<()> {
    let local_tree = walking_diff::tree::Tree::from_path_and_sizes(paths);
    let remote_tree = walking_diff::tree::RemoteTree::from_remote(cli, root, ignore)?;

    walking_diff::diff::diff(&local_tree, &remote_tree, ops, |local, remote| {
        local.size != remote.size
        /*{
           return true;
        }
        // CHECK MD5, IF SAME SIZE AND DIFF MD5, RETURN TRUE ELSE FALSE
        false*/
    });

    Ok(())
}
