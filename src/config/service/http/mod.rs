use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use default_routes::DefaultRoutes;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub mod default_routes;
pub mod initializer;
pub mod middleware;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpServiceConfig {
    #[serde(flatten)]
    #[validate(nested)]
    pub address: Address,
    #[serde(default)]
    #[validate(nested)]
    pub middleware: Middleware,
    #[serde(default)]
    #[validate(nested)]
    pub initializer: Initializer,
    #[serde(default)]
    #[validate(nested)]
    pub default_routes: DefaultRoutes,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Address {
    pub host: String,
    pub port: u32,
}

impl Address {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
