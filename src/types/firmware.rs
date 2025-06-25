use anyhow::{Context, Result, bail};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use url::Url;

use super::directory::{
    Directory, Id, MOMENTUM_DIRECTORY, OFFICIAL_DIRECTORY, UNLEASHED_DIRECTORY, Version,
};

#[derive(Debug)]
pub enum Firmware {
    Official(Id),
    Momentum(Id),
    Unleashed(Id),
    // TODO: Rougemaster, the API is not standard
    // RougeMaster,
    Custom(String),
}

impl Default for Firmware {
    fn default() -> Self {
        Self::Official(Id::Release)
    }
}

impl Serialize for Firmware {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Firmware::Official(channel) => &format!("official@{channel}"),
            Firmware::Momentum(channel) => &format!("momentum@{channel}"),
            Firmware::Unleashed(channel) => &format!("unleashed@{channel}"),

            // Firmware::RougeMaster => "rougemaster",
            Firmware::Custom(url) => url,
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Firmware {
    fn deserialize<D>(deserializer: D) -> Result<Firmware, D::Error>
    where
        D: Deserializer<'de>,
    {
        let firmware = String::deserialize(deserializer)?;

        if let Some((source, channel)) = firmware.split_once('@') {
            let channel = match channel {
                "release" => Id::Release,
                "release-candidate" => Id::ReleaseCanidate,
                "development" => Id::Development,
                _ => todo!(),
            };

            match source {
                "official" => Ok(Firmware::Official(channel)),
                "momentum" => Ok(Firmware::Momentum(channel)),
                "unleashed" => Ok(Firmware::Unleashed(channel)),
                _ => todo!(),
            }
        } else {
            let url = Url::parse(&firmware)
                .map_err(|err| de::Error::custom(format!("{}: {:?}", err, firmware)))?;
            Ok(Firmware::Custom(url.to_string()))
        }
    }
}

impl Firmware {
    pub async fn fetch_manifest(&self) -> Result<Version> {
        match self {
            /*Firmware::Custom(raw) => Ok((
                None,
                Url::parse(raw).with_context(|| format!("parsing custom URL `{}`", raw))?,
            )),*/
            Firmware::Custom(_) => bail!("fetch_manifest called on a custom firmware variant"),

            // All published variants follow the same directory.json spec
            Firmware::Official(ch) | Firmware::Unleashed(ch) | Firmware::Momentum(ch) => {
                let base =
                    Url::parse(self.get_directory()).context("parsing base directory URL")?;
                let dir = Directory::fetch(base).await?;
                let ver = dir
                    .channel_latest_version(ch)
                    .cloned()
                    .with_context(|| format!("no latest version for channel `{}`", ch))?;

                Ok(ver)
            }
        }
    }

    fn get_directory(&self) -> &'static str {
        match self {
            Firmware::Official(_) => OFFICIAL_DIRECTORY,
            Firmware::Unleashed(_) => UNLEASHED_DIRECTORY,
            Firmware::Momentum(_) => MOMENTUM_DIRECTORY,
            Firmware::Custom(_) => unreachable!("matched in fetch_manifest, error taken care of."),
        }
    }
}
