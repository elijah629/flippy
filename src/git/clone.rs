use std::{borrow::Cow, ffi::OsStr};

use anyhow::Context;
use gix::{NestedProgress, Repository, prepare_clone, remote::fetch::Status};

use crate::git::fetch::print_updates;

pub fn clone<P>(
    url: impl AsRef<OsStr>,
    directory: Option<impl Into<std::path::PathBuf>>,
    mut progress: P,
) -> anyhow::Result<Repository>
where
    P: NestedProgress,
    P::SubProgress: 'static,
{
    let url: gix::Url = url.as_ref().try_into()?;
    let directory = directory.map_or_else(
        || {
            let path = gix::path::from_bstr(Cow::Borrowed(url.path.as_ref()));
            if path.extension() == Some(OsStr::new("git")) {
                path.file_stem().map(Into::into)
            } else {
                path.file_name().map(Into::into)
            }
            .context("Filename extraction failed - path too short")
        },
        |dir| Ok(dir.into()),
    )?;

    let mut prepare = prepare_clone(url, directory)?;

    let (mut checkout, fetch_outcome) =
        prepare.fetch_then_checkout(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

    let (repo, outcome) = checkout.main_worktree(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

    match fetch_outcome.status {
        Status::NoPackReceived { .. } => {
            progress.info("The cloned repository appears to be empty".to_string());
        }
        Status::Change {
            update_refs,
            negotiate,
            ..
        } => {
            let remote = repo
                .find_default_remote(gix::remote::Direction::Fetch)
                .expect("one origin remote")?;
            let ref_specs = remote.refspecs(gix::remote::Direction::Fetch);
            print_updates(
                &repo,
                &negotiate,
                update_refs,
                ref_specs,
                fetch_outcome.ref_map,
                &mut progress,
            )?;
        }
    }

    let collisions = outcome.collisions;
    let errors = outcome.errors;

    if !(collisions.is_empty() && errors.is_empty()) {
        let mut messages = Vec::new();
        if !errors.is_empty() {
            messages.push(format!("kept going through {} errors(s)", errors.len()));
            for record in errors {
                progress.info(format!("error: {}: {}", record.path, record.error));
            }
        }
        if !collisions.is_empty() {
            messages.push(format!("encountered {} collision(s)", collisions.len()));
            for col in collisions {
                progress.info(format!(
                    "error: {}: collision ({:?})",
                    col.path, col.error_kind
                ));
            }
        }
        progress.fail(format!(
            "One or more errors occurred - checkout is incomplete: {}",
            messages.join(", "),
        ));
    }

    progress.done("success".to_string());

    Ok(repo)
}
