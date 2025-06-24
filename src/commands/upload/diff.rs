use crate::Flip;
use crate::commands::upload::Commit;
use crate::commands::upload::Path;
use crate::commands::upload::SerialRpcTransport;
use crate::commands::upload::bail;
use crate::commands::upload::info;
use crate::commands::upload::join_as_relative;
use crate::commands::upload::open;
use crate::commands::upload::pathspec::pathspec_from_pattern;
use crate::git::diff::diff_from_head;
use crate::types::mapping::MappingInfo;
use crate::types::remote_sync_file::Repo;
use crate::types::remote_sync_file::SyncFile;
use crate::walking_diff;
use crate::walking_diff::diff::Op;
use anyhow::Context;
use anyhow::Result;
use flipper_rpc::fs::FsMd5;
use flipper_rpc::fs::helpers::os_str_to_str;
use gix::Pathspec;
use gix::bstr::ByteSlice;
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

    let local = local_root.as_ref();
    let remote = remote_root.as_ref();

    info!("Diffing");
    walking_diff::diff::diff(
        &local_tree,
        &remote_tree,
        ops,
        |path, local_size, remote_size| {
            Ok(local_size != remote_size || {
                let local = std::fs::read(join_as_relative(local, path))?;
                let local_hash = md5::compute(local);

                let local_hash = format!("{local_hash:x}");
                let remote_hash = cli.fs_md5(join_as_relative(remote, path))?;

                // TODO: Cache results into the store somehow
                let diff = remote_hash != local_hash;
                info!(
                    "diff CHKSM {local_hash} <-l/r-> {remote_hash} {}",
                    if diff { '❌' } else { '✅' }
                );
                diff
            })
        },
    )?;

    Ok(())
}
