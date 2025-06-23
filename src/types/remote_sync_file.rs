//! Remote syncronization file for the commit hashes and UUIDs of repositories on the remote device
//! SAFTEY: This function will error if it recieves invalid data, assuring no memory corruption can
//! happen

use anyhow::{Result, bail};
use flipper_rpc::{fs::FsRead, transport::serial::rpc::SerialRpcTransport};
use uuid::Uuid;

pub const SYNC_FILE_PATH: &str = "/ext/.flippy_do_not_remove";
const VERSION: u8 = 1;
const NOTICE: &[u8] = b"FLIPPY SYNC FILE: DO NOT MODIFY.";
const NOTICE_LENGTH: usize = NOTICE.len();

// Header layout: u8 + [u8; NOTICE_LENGTH], packed to avoid padding
//#[repr(C, packed)]
//struct HeaderRaw {
//    version: u8,
//    notice: [u8; NOTICE_LENGTH],
//}

/// Repo layout: 16-byte UUID + 20-byte hash, packed
#[repr(C, packed)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    pub uuid: [u8; 16],
    pub hash: [u8; 20],
}

pub struct SyncFile {
    pub repositories: Vec<Repo>,
}

impl SyncFile {
    /// Serializes the SyncFile into a byte vector.
    /// NOTE: When there is only one repository, the length of the output is 69...
    /// NOTE: Nice!
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            1 + NOTICE_LENGTH + self.repositories.len() * std::mem::size_of::<Repo>(),
        );
        // version
        buf.push(VERSION);

        // notice
        buf.extend_from_slice(NOTICE);

        // all repos
        for repo in &self.repositories {
            buf.extend_from_slice(&repo.uuid);
            buf.extend_from_slice(&repo.hash);
        }

        buf
    }

    /// Deserializes a byte slice into a SyncFile.
    ///
    /// Returns Err if version or notice mismatches, or if length is invalid.
    pub fn deserialize(data: impl AsRef<[u8]>) -> Result<Self> {
        let data = data.as_ref();

        // Must at least contain version + notice
        if data.len() < 1 + NOTICE_LENGTH {
            bail!("failed to deserialize sync file: data too short");
        }

        // parse header
        let version = data[0];
        if version != VERSION {
            bail!(
                "failed to deserialize sync file: unsupported version {}",
                version
            );
        }

        let notice_bytes = &data[1..1 + NOTICE_LENGTH];

        if notice_bytes != NOTICE {
            bail!(
                "failed to deserialize sync file: notice mismatch: got {notice_bytes:?}, expected {NOTICE:?}"
            );
        }

        // the rest must be an exact multiple of Repo size
        let repo_data = &data[1 + NOTICE_LENGTH..];
        let repo_size = std::mem::size_of::<Repo>();
        if repo_data.len() % repo_size != 0 {
            bail!(
                "failed to deserialize sync file: repository data length {} is not a multiple of {}",
                repo_data.len(),
                repo_size
            );
        }

        let count = repo_data.len() / repo_size;
        let mut repositories = Vec::with_capacity(count);

        for i in 0..count {
            let start = i * repo_size;
            let chunk = &repo_data[start..start + repo_size];

            // safe because chunk has exactly the right length
            let mut uuid = [0u8; 16];
            let mut hash = [0u8; 20];
            uuid.copy_from_slice(&chunk[0..16]);
            hash.copy_from_slice(&chunk[16..36]);

            repositories.push(Repo { uuid, hash });
        }

        Ok(SyncFile { repositories })
    }

    pub fn find_hash(&self, uuid: &Uuid) -> Option<&[u8; 20]> {
        let bytes = uuid.as_bytes();

        self.repositories
            .iter()
            .find(|repo| repo.uuid == *bytes)
            .map(|x| &x.hash)
    }
}
