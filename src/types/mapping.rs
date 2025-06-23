use gix::bstr::{BString, ByteVec};
use serde::{Deserialize, Serialize};

/// 3) One field per known mapping type
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Mappings {
    pub ibutton: Option<MappingEntry>,
    pub subghz: Option<MappingEntry>,
    pub badusb: Option<MappingEntry>,
    pub rfid: Option<MappingEntry>,
    pub nfc: Option<MappingEntry>,
    pub ir: Option<MappingEntry>,
}

/// 4) Shared include/exclude lists
#[derive(Debug, Deserialize, Serialize)]
pub struct MappingEntry {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl MappingEntry {
    pub fn patterns(&self) -> impl Iterator<Item = BString> + '_ {
        let inc = self.include.iter().map(|s| BString::from(s.as_str()));
        let exc = self.exclude.iter().map(|s| {
            let mut buf = BString::from(":(exclude)");
            buf.push_str(s);
            buf
        });

        inc.chain(exc)
    }
}

#[derive(Debug)]
pub enum Mapping<'a> {
    SubGHz(&'a MappingEntry),
    Rfid(&'a MappingEntry),
    Nfc(&'a MappingEntry),
    IR(&'a MappingEntry),
    IButton(&'a MappingEntry),
    BadUSB(&'a MappingEntry),
}

#[derive(Debug)]
pub struct MappingInfo<'a> {
    pub patterns: &'a MappingEntry,
    pub destination: &'static str,
    pub ignore: &'static [&'static str],
}

impl Mapping<'_> {
    pub fn info(&self) -> MappingInfo {
        match self {
            Mapping::SubGHz(patterns) => MappingInfo {
                patterns,
                destination: "/ext/subghz",
                ignore: &["assets"],
            },
            Mapping::Nfc(patterns) => MappingInfo {
                patterns,
                destination: "/ext/nfc",
                ignore: &["assets", ".cache"],
            },
            Mapping::BadUSB(patterns) => MappingInfo {
                patterns,
                destination: "/ext/badusb",
                ignore: &["assets", ".badusb.settings"],
            },
            Mapping::Rfid(patterns) => MappingInfo {
                patterns,
                destination: "/ext/lfrfid",
                ignore: &[],
            },
            Mapping::IButton(patterns) => MappingInfo {
                patterns,
                destination: "/ext/ibutton",
                ignore: &[],
            },
            Mapping::IR(patterns) => MappingInfo {
                patterns,
                destination: "/ext/infared",
                ignore: &["assets"],
            },
        }
    }
}

impl Mappings {
    /// Iterate over every `include` and `exclude` path in every defined mapping,
    /// yielding `(true, path)` for includes and `(false, path)` for excludes.
    pub fn iter(&self) -> impl Iterator<Item = Mapping> {
        let ibutton = self.ibutton.iter().map(Mapping::IButton);
        let subghz = self.subghz.iter().map(Mapping::SubGHz);
        let badusb = self.badusb.iter().map(Mapping::BadUSB);
        let rfid = self.rfid.iter().map(Mapping::Rfid);
        let nfc = self.nfc.iter().map(Mapping::Nfc);
        let ir = self.ir.iter().map(Mapping::IR);

        // chain them all into one lazy iterator
        ibutton
            .chain(subghz)
            .chain(badusb)
            .chain(rfid)
            .chain(nfc)
            .chain(ir)
    }
}
