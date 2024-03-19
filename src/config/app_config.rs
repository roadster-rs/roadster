use anyhow::anyhow;
use config::Config;
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};

use crate::config::environment::Environment;
use crate::config::middleware::Middleware;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    pub app: App,
    pub server: Server,
    pub tracing: Tracing,
    pub environment: Environment,
    #[serde(default)]
    pub middleware: Middleware,
}

impl AppConfig {
    pub fn new() -> anyhow::Result<Self> {
        dotenv().ok();

        let environment = Environment::new()?;
        let environment: &'static str = environment.into();

        let config: AppConfig = Config::builder()
            .add_source(config::File::with_name("config/default.toml"))
            .add_source(config::File::with_name(&format!(
                "config/{environment}.toml"
            )))
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
            .map_err(|err| anyhow!("Unable to deserialize app config: {err:?}"))?;

        // Validations
        // Todo: Is there a crate that would make this easier?
        debug_assert_eq!(
            config.middleware.set_request_id.custom.common.header_name,
            config
                .middleware
                .propagate_request_id
                .custom
                .common
                .header_name,
            "A different request ID header name is used when handling a request vs when sending a response."
        );

        Ok(config)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct App {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Server {
    pub host: String,
    pub port: u32,
}

impl Server {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Tracing {
    pub level: String,
}
