use std::{fmt::Display, path::Path};

use anyhow::{Context, Result, bail};
use reqwest::header::CONTENT_LENGTH;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::progress::progress;

pub const OFFICIAL_DIRECTORY: &str = "https://update.flipperzero.one/firmware/directory.json";
pub const UNLEASHED_DIRECTORY: &str = "https://up.unleashedflip.com/directory.json";
pub const MOMENTUM_DIRECTORY: &str = "https://up.momentum-fw.dev/firmware/directory.json";

#[derive(Deserialize, Debug, Clone)]
pub struct Directory {
    pub channels: Vec<Channel>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Channel {
    pub id: Id,
    pub title: String,
    pub description: String,
    pub versions: Option<Vec<Version>>,
}
#[derive(Deserialize, Debug, Clone)]
pub struct Version {
    pub version: String,
    pub changelog: String,
    pub timestamp: u64,
    pub files: Vec<File>,
}

impl Version {
    pub fn latest_tgz(&self) -> Result<&File> {
        self.files
            .iter()
            .find(|f| f.file_type == "update_tgz")
            .with_context(|| format!("no `update_tgz` file found in version {}", self.version))
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version: {}", self.version)?;
        writeln!(
            f,
            "Released on: {}",
            jiff::Timestamp::from_second(self.timestamp as i64)
                .expect("timestamp could not be parsed")
        )?;

        let len = self.changelog.len();

        const MAX_LINES: usize = 10;
        writeln!(f, "{:-^40}", " Changelog ")?;
        if len > MAX_LINES {
            let lines = self.changelog.lines().take(10);

            for line in lines {
                writeln!(f, "{}", line)?;
            }

            writeln!(f, "...")?;
        } else {
            writeln!(f, "{}", self.changelog)?;
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct File {
    pub url: Url,
    pub target: Target,

    #[serde(alias = "type")]
    pub file_type: String,
    pub sha256: String,
}

impl File {
    pub async fn download(&self, out: impl AsRef<Path>) -> Result<()> {
        use futures_util::StreamExt;
        use std::str::FromStr;

        let (progress, handle) = progress();

        let item = progress.add_child("downloading firmware .tgz");
        let client = reqwest::Client::new();
        let url = self.url.as_str();
        let response = client.head(url).send().await?;

        let length = response
            .headers()
            .get(CONTENT_LENGTH)
            .context("response doesn't include the content length")?;

        let length = u64::from_str(length.to_str().context("Invalid Content-Length header")?)?;

        let mut stream = client.get(url).send().await?.bytes_stream();

        item.init(
            Some(length as usize),
            Some(prodash::unit::dynamic_and_mode(
                prodash::unit::Bytes,
                prodash::unit::display::Mode::with_throughput(),
            )),
        );

        let mut file = tokio::fs::File::create(out).await?;
        let mut hasher = Sha256::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            item.inc_by(chunk.len());
            file.write_all(&chunk).await?;
            hasher.update(&chunk);
        }

        let actual = hex::encode(hasher.finalize());

        if actual != self.sha256 {
            bail!("hash mismatch, expected {}, got {actual}", self.sha256);
        }

        handle.shutdown_and_wait();

        Ok(())
        /*
        println!("{response:?}");
        let length = response.content_length().map(|x| x as usize);


        let mut buffer = Vec::with_capacity(length.expect("Cannot find content length!"));
        while let Some(bytes_read) = response.().await? {
            buffer.extend_from_slice(&bytes_read.slice(..bytes_read.len()));

            item.inc_by(bytes_read.len());
        }

        item.done("Fetched directory");


        let json = serde_json::from_slice(&buffer)?;

        Ok(json)*/
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:-^40}", "Firmware file")?;
        writeln!(f, "url: {}", self.url)?;
        writeln!(f, "sha256-{}", self.sha256)?;

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize)]
pub enum Id {
    #[serde(rename = "development")]
    Development,
    #[serde(rename = "release-candidate")]
    ReleaseCanidate,
    #[serde(rename = "release")]
    Release,
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Release => "release",
            Self::ReleaseCanidate => "release-candidate",
            Self::Development => "development",
        })?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub enum Target {
    #[serde(rename = "f7")]
    F7,
    #[serde(rename = "f18")]
    F18,
    #[serde(rename = "any")]
    Any,
}

impl Directory {
    pub async fn fetch(url: Url) -> Result<Self> {
        //let (progress, handle) = progress();

        //let mut item = progress.add_child("fetching directory");

        let response = reqwest::get(url).await?.json().await?;
        Ok(response)
        /*
        println!("{response:?}");
        let length = response.content_length().map(|x| x as usize);

        item.init(
            length,
            Some(prodash::unit::dynamic_and_mode(
                prodash::unit::Bytes,
                prodash::unit::display::Mode::with_throughput(),
            )),
        );

        let mut buffer = Vec::with_capacity(length.expect("Cannot find content length!"));
        while let Some(bytes_read) = response.().await? {
            buffer.extend_from_slice(&bytes_read.slice(..bytes_read.len()));

            item.inc_by(bytes_read.len());
        }

        item.done("Fetched directory");

        handle.shutdown_and_wait();

        let json = serde_json::from_slice(&buffer)?;

        Ok(json)*/
    }

    pub fn channel_latest_version(&self, channel: &Id) -> Option<&Version> {
        let channel = self.channels.iter().find(|&x| x.id.eq(channel))?;

        let versions = &channel
            .versions
            .as_ref()
            .expect("No versions available for the selected channel");

        // At the moment, i have not seen a channel with multiple versions.
        let version = &versions[0];

        Some(version)
    }
}
