use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub address: String,
    pub org: String,
    pub bucket: String,
    pub token: String,
}
