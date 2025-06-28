use crate::flipper::pick_cli;
use crate::progress::progress;
use crate::{
    commands::upload::diff::diff_all_repositories, types::remote_sync_file::SYNC_FILE_PATH,
};
use crate::{
    types::{flip::Flip, remote_sync_file::SyncFile},
    walking_diff::diff::Op,
};
use anyhow::{Result, bail};
use cliclack::confirm;
use flipper_rpc::fs::{FsCreateDir, FsRead, FsRemove, FsWrite};
use gix::{Commit, open};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use tokio::fs;
use tracing::{info, instrument, warn};

mod diff;
mod pathspec;

#[instrument]
pub async fn run(flip: Flip, force_walkdir: bool) -> Result<()> {
    if force_walkdir {
        unimplemented!(
            "--force-walkdir is not fully implemented yet. Please delete {SYNC_FILE_PATH} manually, then rerun this command."
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
                "If it is not your first time, please take care in keeping that file save and make a backup of it. The sync file holds important information to drastically improve transfer speeds and I/O calls. Over time many I/O calls will wear out your SD card."
            );

            info!("Creating blank sync file, rerun command for walking-diff");
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

    let (progress, handle) = progress();

    let mut item = progress.add_child("operating");

    item.init(Some(count), None);

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
                let child = item.add_child(format!("copy {from:?} -> {to:?}"));

                let from = fs::read(from).await?;

                let (tx, rx) = channel();

                let handle = tokio::spawn(async move {
                    for sent in rx {
                        child.set(sent);
                    }
                });

                cli.fs_write(to, from, Some(tx))?;

                handle.await?;

                item.inc();
            }
            Op::CreateDir(path_buf) => {
                let to = mapping_root_remote.join(path_buf);

                cli.fs_create_dir(to)?;
                item.inc();
            }
            Op::Remove(path_buf) => {
                let to = mapping_root_remote.join(path_buf);
                cli.fs_remove(to, true)?;

                item.inc();
            }
        };
    }

    item.done(format!(
        "Successfully completed {} operations",
        operations.len()
    ));

    handle.shutdown_and_wait();

    // If you select NO on the confirm, running the command again will make itself think that it
    // ran the last time, desyncing the commit hash
    // Only update if it didnt fail (likely in beta)
    cli.fs_write(SYNC_FILE_PATH, updated_sync_file.serialize(), None)?;

    Ok(())
}
