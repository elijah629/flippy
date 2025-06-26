use std::{
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::mpsc::channel,
    time::Instant,
};

use crate::{
    flipper::pick_cli,
    types::{firmware::Firmware, flip::Flip},
};
use anyhow::{Context, bail};
use cliclack::confirm;
use flate2::read::GzDecoder;
use flipper_rpc::fs::{FsCreateDir, FsRemove, FsTarExtract, FsWrite};
use serde::{
    Deserialize,
    de::{self, IntoDeserializer, value::StringDeserializer},
};
use tar::Archive;
use tokio::io::AsyncReadExt;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn set(mut flip: Flip, firmware: String) -> anyhow::Result<()> {
    let deserializer: StringDeserializer<de::value::Error> = firmware.into_deserializer();

    flip.firmware = Firmware::deserialize(deserializer)?;

    flip.write().await?;

    Ok(())
}

pub async fn update(flip: Flip) -> anyhow::Result<()> {
    let firmware = flip.firmware;

    let version = firmware.fetch_manifest().await?;
    let firmware_file = version.latest_tgz()?;

    let url = &firmware_file.url;

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
        info!("Fetched version info");

        println!("{version}");
        println!("{firmware_file}");

        if !confirm("OK to download?").interact()? {
            bail!("Aborted");
        }

        tokio::fs::create_dir(&store_path).await?;
        firmware_file.download(&tgz_path).await?;
    }

    let mut tar_gz = std::fs::File::open(&tgz_path)?;

    let mut buf = vec![];
    tar_gz.read_to_end(&mut buf)?;
    let reader = Cursor::new(&buf);

    let tar = GzDecoder::new(reader);
    let mut archive = Archive::new(tar);
    let update_name = archive
        .entries()?
        .next()
        .context("Invalid .tgz, no items")??;
    let update_name = update_name.path()?;
    let update_dir = Path::new("/ext/update")
        .join(update_name)
        .components()
        .collect::<PathBuf>();

    let mut cli = pick_cli()?;

    let tgz_path = update_dir.join(tgz_name);
    /* cli.fs_create_dir(&update_dir)?;


    let (tx, rx) = channel();

    let handle = std::thread::spawn(move || {
        let start = Instant::now();
        for (sent, total) in rx {
            println!("[+{:.2?}] Progress: {}/{}", start.elapsed(), sent, total);
        }
    });

    println!("{tgz_path:?}");
    cli.fs_write(&tgz_path, buf, Some(tx))?;

    handle.join().unwrap();*/

    //    println!("{update_dir:?}");
    //cli.fs_extract_tgz(&tgz_path, update_dir.join("a"))?;

    // println!("{tgz_path:?}");
    //cli.fs_remove(&tgz_path, false)?;
    //cli.fs_create_dir(update_dir)?;

    /*  for file in archive.entries()? {
        let mut file = file?;
        let path = file.path()?;
        // If the last char is a `/`, it is a dir. / == 47
        let is_dir = file.path_bytes().last() == Some(&47);
        let out_path = PathBuf::from("/ext/update").join(path);

        if is_dir {
            let out_path = out_path.to_str().unwrap().strip_suffix("/").unwrap();
            cli.fs_create_dir(out_path)?;
        } else {
            let mut buf = vec![];
            file.read_to_end(&mut buf)?;

            cli.fs_write(out_path, buf)?;
        }
    }*/
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
