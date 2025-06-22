use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::{firmware::Firmware, repository::Repository};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, trace};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Flip {
    #[serde(skip)]
    pub source_path: PathBuf,

    pub name: String,
    // pub path: PathBuf,
    pub firmware: Firmware,
    pub repositories: HashMap<String, Repository>,
}

impl Flip {
    pub async fn exists<P: AsRef<Path>>(path: P) -> anyhow::Result<bool> {
        let exists = fs::try_exists(path.as_ref().join("flip.toml")).await?;

        Ok(exists)
    }

    pub async fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();

        debug!("attempting to parse flip.toml from a path");
        trace!("reading path {} as flip.toml", path.display());
        let content = fs::read_to_string(path.join("flip.toml")).await?;
        let mut flip: Flip = toml::from_str(&content)?;

        flip.source_path = path.to_path_buf();

        Ok(flip)
    }

    pub async fn write(self) -> anyhow::Result<()> {
        debug!("writing to flip.toml @ {}", self.source_path.display());

        Ok(fs::write(
            self.source_path.join("flip.toml"),
            toml::to_string_pretty(&self)?,
        )
        .await?)
    }
}
