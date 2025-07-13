use crate::config::environment::Environment;
use crate::config::service::common::address::Address;
use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use crate::error::RoadsterResult;
use config::{FileFormat, FileSourceString};
use default_routes::DefaultRoutes;
use serde_derive::{Deserialize, Serialize};
use url::Url;
use validator::Validate;

pub mod default_routes;
pub mod initializer;
pub mod middleware;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("config/default.toml"), FileFormat::Toml)
}

pub(crate) fn default_config_per_env(
    environment: Environment,
) -> Option<config::File<FileSourceString, FileFormat>> {
    let config = match environment {
        Environment::Production => Some(include_str!("config/production.toml")),
        Environment::Development => Some(include_str!("config/development.toml")),
        _ => None,
    };
    config.map(|c| config::File::from_str(c, FileFormat::Toml))
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct HttpServiceConfig {
    /// A URL for the domain where the service is hosted, if available.
    pub url: Option<Url>,
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

impl HttpServiceConfig {
    /// Get either the URL for the domain where the service is hosted if available, or the URL for
    /// the socket address where the service is running locally.  
    pub fn url(&self) -> RoadsterResult<Url> {
        if let Some(url) = self.url.as_ref() {
            Ok(url.clone())
        } else {
            Ok(self.address.url_with_scheme().parse()?)
        }
    }
}
