use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::mapping::Mapping;

#[derive(Serialize, Deserialize, Debug)]
pub struct Repository {
    pub url: String,
    pub uuid: Uuid,
    pub mappings: Vec<Mapping>,
}
