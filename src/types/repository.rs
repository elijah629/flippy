use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::mapping::Mappings;

#[derive(Serialize, Deserialize, Debug)]
pub struct Repository {
    pub url: String,
    pub uuid: Uuid,
    pub mappings: Mappings,
}
