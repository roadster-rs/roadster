use crate::config::environment::Environment;
use crate::config::service::common::address::Address;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("config/default.toml"), FileFormat::Toml)
}

pub(crate) fn default_config_per_env(
    environment: Environment,
) -> Option<config::File<FileSourceString, FileFormat>> {
    let config = match environment {
        Environment::Production => Some(include_str!("config/production.toml")),
        _ => None,
    };
    config.map(|c| config::File::from_str(c, FileFormat::Toml))
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct GrpcServiceConfig {
    #[serde(flatten)]
    #[validate(nested)]
    pub address: Address,
}
