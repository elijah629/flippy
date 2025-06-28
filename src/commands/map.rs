use std::path::PathBuf;

use anyhow::Context;
use tracing::{debug, instrument, trace};

use crate::types::{flip::Flip, mapping::MappingEntry};

#[instrument]
pub async fn run(
    mut flip: Flip,
    db_type: String,
    repo: String,
    path: PathBuf,
    excludes: bool,
) -> anyhow::Result<()> {
    debug!("Checking if repo {repo} exists");

    let repo = flip
        .repositories
        .get_mut(&repo)
        .context(format!("repository {} not found", repo))?;

    trace!("Pushing database mapping to list");

    let mapping = match db_type.as_str() {
        "subghz" => &mut repo.mappings.subghz,
        "rfid" => &mut repo.mappings.rfid,
        "nfc" => &mut repo.mappings.nfc,
        "ir" => &mut repo.mappings.ir,
        "ibutton" => &mut repo.mappings.ibutton,
        "badusb" => &mut repo.mappings.badusb,
        _ => unreachable!("matched by clap"),
    };

    if excludes {
        add_exclude(mapping, path);
    } else {
        add_include(mapping, path);
    }

    flip.write().await?;

    Ok(())
}

#[instrument]
fn add_include(mapping: &mut Option<MappingEntry>, path: PathBuf) {
    let path = path.to_string_lossy().to_string();

    match mapping {
        Some(existing) => {
            existing.include.push(path);
        }
        None => {
            *mapping = Some(MappingEntry {
                include: vec![path],
                exclude: vec![],
            })
        }
    }
}

#[instrument]
fn add_exclude(mapping: &mut Option<MappingEntry>, path: PathBuf) {
    let path = format!(":(exclude){}", path.to_string_lossy());

    match mapping {
        Some(existing) => {
            existing.exclude.push(path);
        }
        None => {
            *mapping = Some(MappingEntry {
                include: vec![],
                exclude: vec![path],
            })
        }
    }
}
