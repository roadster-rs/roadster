use crate::config::environment::Environment;
use crate::config::service::common::address::Address;
use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use config::{FileFormat, FileSourceString};
use default_routes::DefaultRoutes;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub mod default_routes;
pub mod initializer;
pub mod middleware;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
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
pub struct HttpServiceConfig {
    #[serde(flatten)]
    #[validate(nested)]
    pub address: Address,
    #[validate(nested)]
    pub middleware: Middleware,
    #[validate(nested)]
    pub initializer: Initializer,
    #[validate(nested)]
    pub default_routes: DefaultRoutes,
}
