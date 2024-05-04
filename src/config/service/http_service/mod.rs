use crate::config::service::http_service::initializer::Initializer;
use crate::config::service::http_service::middleware::Middleware;
use serde_derive::{Deserialize, Serialize};

pub mod initializer;
pub mod middleware;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpServiceConfig {
    #[serde(flatten)]
    pub address: Address,
    #[serde(default)]
    pub middleware: Middleware,
    #[serde(default)]
    pub initializer: Initializer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
