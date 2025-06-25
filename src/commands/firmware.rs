use crate::{
    flipper::pick_cli,
    types::{firmware::Firmware, flip::Flip},
};
use anyhow::bail;
use cliclack::confirm;
use flate2::read::GzDecoder;
use flipper_rpc::fs::FsWrite;
use serde::{
    Deserialize,
    de::{self, IntoDeserializer, value::StringDeserializer},
};
use tar::Archive;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn set(mut flip: Flip, firmware: String) -> anyhow::Result<()> {
    let deserializer: StringDeserializer<de::value::Error> = firmware.into_deserializer();

    flip.firmware = Firmware::deserialize(deserializer)?;

    flip.write().await?;

    Ok(())
}

pub async fn pull(flip: Flip) -> anyhow::Result<()> {
    let firmware = flip.firmware;

    let version = firmware.fetch_manifest().await?;
    let firmware_file = version.latest_tgz()?;

    info!("Fetched version info");

    println!("{version}");
    println!("{firmware_file}");

    if !confirm("OK to download?").interact()? {
        bail!("Aborted");
    }

    let url = &firmware_file.url;

    let store_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, url.as_str().as_bytes());
    let store_path = flip.source_path.join("store").join(store_id.to_string());

    if tokio::fs::try_exists(&store_path).await? {
        warn!("This firmware version has already been pulled, removing.");
        tokio::fs::remove_dir_all(&store_path).await?;
    }

    let tgz_path = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap();
    let tgz_path = store_path.join(tgz_path);

    tokio::fs::create_dir(&store_path).await?;

    firmware_file.download(&tgz_path).await?;

    let tar_gz = std::fs::File::open(&tgz_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(store_path)?;

    // let cli = pick_cli()?;
    /*
        *
    Overview:

    Put files in /ext/update/name-of-update
    UPDATE(Manifest.fuf)
    REboot(Mode::Update)

    (i think)
        */

    /*  for entry in to_copy {
        let entry = entry?;

        let path = entry.path()?;
        println!("{path:?}");

        //cli.fs_write(path, data);
        //entry

        // cli.fs_write(path, data)
    }*/

    /*cli.send_and_receive(flipper_rpc::rpc::req::Request::SystemUpdate(UpdateRequest {
        update_manifest: ""
    });*/

    Ok(())
}
