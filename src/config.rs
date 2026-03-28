use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::sources::{FirmwareSource, RepositorySource};

pub const CONFIG_FILE_NAME: &str = "flip.toml";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FlipConfig {
    pub name: String,
    #[serde(with = "firmware_source_format")]
    pub firmware: FirmwareSource,
    #[serde(default)]
    pub repositories: BTreeMap<String, RepositoryConfig>,
}

impl FlipConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            firmware: FirmwareSource::default(),
            repositories: BTreeMap::new(),
        }
    }

    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = config_path(dir);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read '{}'", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("failed to parse '{}'", path.display()))
    }

    pub fn save_to_dir(&self, dir: &Path) -> Result<()> {
        let path = config_path(dir);
        let raw = toml::to_string_pretty(self)
            .with_context(|| format!("failed to serialize '{}'", path.display()))?;
        fs::write(&path, raw).with_context(|| format!("failed to write '{}'", path.display()))
    }

    pub fn next_repository_name(&self, base_name: String) -> String {
        if !self.repositories.contains_key(&base_name) {
            return base_name;
        }

        let mut index = 2_u32;
        loop {
            let candidate = format!("{base_name}-{index}");
            if !self.repositories.contains_key(&candidate) {
                return candidate;
            }
            index += 1;
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RepositoryConfig {
    #[serde(with = "repository_source_format")]
    pub source: RepositorySource,
}

fn config_path(dir: &Path) -> PathBuf {
    dir.join(CONFIG_FILE_NAME)
}

mod firmware_source_format {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::sources::FirmwareSource;

    pub fn serialize<S>(value: &FirmwareSource, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<FirmwareSource, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

mod repository_source_format {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::sources::RepositorySource;

    pub fn serialize<S>(value: &RepositorySource, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RepositorySource, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{FlipConfig, RepositoryConfig};
    use crate::sources::{FirmwarePreset, FirmwareSource, RepositoryRemote, RepositorySource};

    #[test]
    fn round_trips_flip_toml() {
        let temp_dir = tempdir().unwrap();
        let mut config = FlipConfig::new("demo".to_owned());
        config.firmware = FirmwareSource::Preset(FirmwarePreset::Momentum);
        config.repositories.insert(
            "firmware".to_owned(),
            RepositoryConfig {
                source: RepositorySource::Remote(RepositoryRemote::ScpLike(
                    "git@github.com:example/project.git".to_owned(),
                )),
            },
        );

        config.save_to_dir(temp_dir.path()).unwrap();
        let loaded = FlipConfig::load_from_dir(temp_dir.path()).unwrap();

        assert_eq!(loaded, config);
    }

    #[test]
    fn generates_unique_repository_names() {
        let mut config = FlipConfig::new("demo".to_owned());
        config.repositories.insert(
            "irdb".to_owned(),
            RepositoryConfig {
                source: RepositorySource::Local("Flipper-IRDB".into()),
            },
        );

        assert_eq!(config.next_repository_name("irdb".to_owned()), "irdb-2");
    }
}
