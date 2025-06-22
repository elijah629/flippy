use cliclack::confirm;
use serde::{
    Deserialize,
    de::{self, IntoDeserializer, value::StringDeserializer},
};
use tracing::info;

use crate::types::{firmware::Firmware, flip::Flip};

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

    if confirm("OK to download?").interact()? {
        println!("Def downloading");
    }
    Ok(())
}
