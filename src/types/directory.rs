use std::{fmt::Display, path::Path};

use crate::progress::progress;
use anyhow::{Context, Result, bail};
use reqwest::header::CONTENT_LENGTH;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncWriteExt, BufWriter};
use url::Url;

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

    #[allow(dead_code)]
    pub title: String,

    #[allow(dead_code)]
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

        let ts = jiff::Timestamp::from_second(self.timestamp as i64)
            .map(|t| t.to_string())
            .unwrap_or_else(|_| "Invalid timestamp".to_string());

        writeln!(f, "Released on: {}", ts)?;

        writeln!(f)?;
        writeln!(f, "Changelog")?;

        const MAX_LINES: usize = 10;
        let lines: Vec<&str> = self.changelog.lines().collect();

        if lines.len() > MAX_LINES {
            for line in &lines[..MAX_LINES] {
                writeln!(f, "{}", line)?;
            }
            writeln!(f, "...")?;
        } else {
            for line in lines {
                writeln!(f, "{}", line)?;
            }
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
    pub async fn download(url: &Url, sha256: Option<&str>, out: impl AsRef<Path>) -> Result<()> {
        use futures_util::StreamExt;
        use std::str::FromStr;

        let (progress, handle) = progress();
        let item = progress.add_child("downloading firmware .tgz");

        let client = reqwest::Client::new();
        let url = url.as_str();

        // Fetch content length for progress and preallocation
        let response = client.head(url).send().await?;
        let length = response
            .headers()
            .get(CONTENT_LENGTH)
            .context("response doesn't include the content length")?;
        let length = u64::from_str(length.to_str().context("Invalid Content-Length header")?)?;

        // Start download
        let mut stream = client.get(url).send().await?.bytes_stream();

        item.init(
            Some(length as usize),
            Some(prodash::unit::dynamic_and_mode(
                prodash::unit::Bytes,
                prodash::unit::display::Mode::with_throughput(),
            )),
        );

        let file = tokio::fs::File::create(&out).await?;
        file.set_len(length).await?; // Pre-allocate the full file
        let mut file = BufWriter::new(file);
        let mut hasher = Sha256::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            item.inc_by(chunk.len());
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }

        file.flush().await?;

        if let Some(sha256) = sha256 {
            let actual = hex::encode(hasher.finalize());

            if actual != sha256 {
                bail!("hash mismatch, expected {}, got {actual}", sha256);
            }
        }

        handle.shutdown_and_wait();
        Ok(())
    }
}

impl Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:-^40}", "Firmware file")?;
        writeln!(f, "\nFirmware File")?;
        writeln!(f, "Type: {}", self.file_type)?;
        writeln!(f, "Target: {}", self.target)?;
        writeln!(f, "URL: {}", self.url)?;
        writeln!(f, "Sha256: {}", self.sha256)?;
        Ok(())
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Target::F7 => "f7",
            Target::F18 => "f18",
            Target::Any => "any",
        };
        write!(f, "{}", s)
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
        let response = reqwest::get(url).await?.json().await?;
        Ok(response)
    }

    pub fn channel_latest_version(&self, channel: &Id) -> Option<&Version> {
        let channel = self.channels.iter().find(|&x| x.id.eq(channel))?;

        let versions = &channel
            .versions
            .as_ref()
            .expect("No versions available for the selected channel");

        // At the moment, i have not seen a channel with multiple versions. The
        // latest version should be first if it is sorted properly anyways.
        let version = &versions[0];

        Some(version)
    }
}
