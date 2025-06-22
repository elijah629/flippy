use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub enum Mapping {
    SubGHz(PathBuf),
    Rfid(PathBuf),
    Nfc(PathBuf),
    IR(PathBuf),
    IButton(PathBuf),
    BadUSB(PathBuf),
}

#[derive(Debug)]
pub struct MappingInfo<'a> {
    pub pattern: &'a PathBuf,
    pub destination: &'static str,
    pub ignore: &'static [&'static str],
}

impl Mapping {
    pub fn info(&self) -> MappingInfo {
        match self {
            Mapping::SubGHz(pattern) => MappingInfo {
                pattern,
                destination: "/ext/subghz",
                ignore: &["assets"],
            },
            Mapping::Nfc(pattern) => MappingInfo {
                pattern,
                destination: "/ext/nfc",
                ignore: &["assets", ".cache"],
            },
            Mapping::BadUSB(pattern) => MappingInfo {
                pattern,
                destination: "/ext/badusb",
                ignore: &["assets", ".badusb.settings"],
            },
            Mapping::Rfid(pattern) => MappingInfo {
                pattern,
                destination: "/ext/lfrfid",
                ignore: &[],
            },
            Mapping::IButton(pattern) => MappingInfo {
                pattern,
                destination: "/ext/ibutton",
                ignore: &[],
            },
            Mapping::IR(pattern) => MappingInfo {
                pattern,
                destination: "/ext/infared",
                ignore: &["assets"],
            },
        }
    }
}
