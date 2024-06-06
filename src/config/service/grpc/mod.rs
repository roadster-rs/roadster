use crate::config::service::common::address::Address;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GrpcServiceConfig {
    #[serde(flatten)]
    #[validate(nested)]
    pub address: Address,
}
