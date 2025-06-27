use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::mpsc::channel,
};

use crate::{
    flipper::pick_cli,
    progress::progress,
    types::{directory::File, firmware::Firmware, flip::Flip},
};
use anyhow::bail;
use cliclack::confirm;
use flate2::read::GzDecoder;
use flipper_rpc::{
    fs::{FsCreateDir, FsRemove, FsWrite, helpers::os_str_to_str},
    proto::system::{UpdateRequest, reboot_request::RebootMode},
    rpc::req::Request,
    transport::Transport,
};
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

/// Overview of what the update operation looks like:
/// - Fetch ,tgz
/// - Extract it
/// - Put all of it's files inside of /ext/update/xxx
/// - Run Update on /ext/update/xxx/update.fuf
/// - Reboot into update mode
pub async fn update(flip: Flip) -> anyhow::Result<()> {
    let firmware = flip.firmware;

    let (url, sha265) = match firmware {
        Firmware::Custom(url) => (url.parse()?, None),
        _ => {
            let version = firmware.fetch_manifest().await?;
            let firmware_file = version.latest_tgz()?;

            println!("{version}");
            println!("{firmware_file}");

            (
                firmware_file.url.clone(),
                Some(firmware_file.sha256.clone()),
            )
        }
    };

    let store_id = Uuid::new_v5(&Uuid::NAMESPACE_URL, url.as_str().as_bytes());
    let store_path = flip.source_path.join("store").join(store_id.to_string());

    let tgz_name = url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .unwrap();
    let tgz_path = store_path.join(tgz_name);

    if tokio::fs::try_exists(&store_path).await? {
        warn!("This firmware version has already been pulled locally, keeping.");
        //tokio::fs::remove_dir_all(&store_path).await?;
    } else {
        if !confirm("OK to download?").interact()? {
            bail!("Aborted");
        }

        tokio::fs::create_dir(&store_path).await?;
        File::download(&url, sha265.as_deref(), &tgz_path).await?;
    }

    let mut tar_gz = std::fs::File::open(&tgz_path)?;

    let mut buf = vec![];
    tar_gz.read_to_end(&mut buf)?;
    let reader = Cursor::new(&buf);

    let (progress, handle) = progress();

    let tar = GzDecoder::new(reader);
    let mut archive = Archive::new(tar);

    let mut cli = pick_cli()?;

    cli.fs_create_dir("/ext/update")?;
    let mut base = None;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let header = entry.header();
        let is_dir = header.entry_type().is_dir();
        let path = Path::new("/ext/update").join(entry.path()?.components().collect::<PathBuf>());

        if is_dir {
            let existed = cli.fs_create_dir(&path)?;
            if existed {
                cli.fs_remove(&path, true)?;
                cli.fs_create_dir(&path)?;
            }
            base = Some(path);
        } else {
            let item = progress.add_child(os_str_to_str(path.file_name().unwrap())?);
            let size = entry.size() as usize;

            item.init(
                Some(size),
                Some(prodash::unit::dynamic_and_mode(
                    prodash::unit::Bytes,
                    prodash::unit::display::Mode::with_throughput(),
                )),
            );

            let mut buf = vec![0u8; size];
            entry.read_exact(&mut buf)?;

            let (tx, rx) = channel();

            let handle = tokio::spawn(async move {
                for sent in rx {
                    item.set(sent);
                }
            });

            cli.fs_write(path, &mut buf, Some(tx))?;

            handle.await?;
        }
    }
    handle.shutdown_and_wait();

    if !confirm("OK to Update? This will restart the flipper.").interact()? {
        bail!("Aborted");
    }

    let manifest = os_str_to_str(base.unwrap().join("update.fuf").as_os_str())?.to_string();

    cli.send_and_receive(Request::SystemUpdate(UpdateRequest {
        update_manifest: manifest,
    }))?;

    cli.send(Request::Reboot(RebootMode::Update))?; // Dont recieve cuz the device just got nuked

    info!(
        "Flipper has been rebooted into update mode, please wait for the device to power back on before attempting further modifications."
    );

    Ok(())
}
