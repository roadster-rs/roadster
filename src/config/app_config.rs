use std::time::Duration;

use anyhow::anyhow;
use config::{Case, Config};
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use url::Url;

use crate::config::environment::Environment;
use crate::config::initializer::Initializer;
use crate::config::middleware::Middleware;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    pub app: App,
    pub server: Server,
    pub tracing: Tracing,
    pub environment: Environment,
    pub database: Database,
    pub worker: Option<Worker>,
    #[serde(default)]
    pub middleware: Middleware,
    #[serde(default)]
    pub initializer: Initializer,
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
            .add_source(
                config::Environment::default()
                    .prefix("roadster")
                    .convert_case(Case::Kebab)
                    .separator("."),
            )
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

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Database {
    /// This can be overridden with an environment variable, e.g. `ROADSTER.DATABASE.URI=postgres://example:example@example:1234/example_app`
    pub uri: Url,
    pub auto_migrate: bool,
    #[serde(default = "Database::default_connect_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub connect_timeout: Duration,
    #[serde(default = "Database::default_acquire_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub acquire_timeout: Duration,
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub idle_timeout: Option<Duration>,
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub max_lifetime: Option<Duration>,
    #[serde(default)]
    pub min_connections: u32,
    pub max_connections: u32,
}

impl Database {
    fn default_connect_timeout() -> Duration {
        Duration::from_millis(1000)
    }

    fn default_acquire_timeout() -> Duration {
        Duration::from_millis(1000)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Worker {
    // Todo: Make Redis optional for workers?
    pub redis: Redis,
    #[serde(default)]
    pub queue_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Redis {
    pub uri: Url,
}
