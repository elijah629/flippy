use std::path::PathBuf;

use anyhow::Context;
use tracing::{debug, trace};

use crate::types::{flip::Flip, mapping::Mapping};

pub async fn run(
    mut flip: Flip,
    db_type: String,
    repo: String,
    path: PathBuf,
) -> anyhow::Result<()> {
    debug!("Checking if repo {repo} exists");

    let repo_mut = flip
        .repositories
        .get_mut(&repo)
        .context(format!("repository {} not found", repo))?;

    trace!("Pushing database mapping to list");
    repo_mut.mappings.push(match db_type.as_str() {
        "subghz" => Mapping::SubGHz(path),
        "rfid" => Mapping::Rfid(path),
        "nfc" => Mapping::Nfc(path),
        "ir" => Mapping::IR(path),
        "ibutton" => Mapping::IButton(path),
        "badusb" => Mapping::BadUSB(path),
        _ => unreachable!(),
    });

    //info!("Successfully mapped {repo}/{} to {db_type}", path.display());

    flip.write().await?;

    Ok(())
}
