#[cfg(feature = "db-sql")]
use std::time::Duration;

use anyhow::anyhow;
use config::{Case, Config};
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "db-sql")]
use serde_with::serde_as;
#[cfg(any(feature = "db-sql", feature = "sidekiq"))]
use url::Url;

use crate::config::environment::Environment;
use crate::config::initializer::Initializer;
use crate::config::middleware::Middleware;
use crate::util::serde_util::{default_true, UriOrString};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    pub app: App,
    pub server: Server,
    pub auth: Auth,
    pub tracing: Tracing,
    pub environment: Environment,
    #[cfg(feature = "db-sql")]
    pub database: Database,
    #[cfg(feature = "sidekiq")]
    pub worker: Worker,
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
    /// Shutdown the whole app if an error occurs in one of the app's top-level tasks (API, workers, etc).
    #[serde(default = "default_true")]
    pub shutdown_on_error: bool,
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
pub struct Auth {
    pub jwt: Jwt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Jwt {
    pub secret: String,
    #[serde(default)]
    pub claims: JwtClaims,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct JwtClaims {
    // Todo: Default to the server URL?
    pub audience: Vec<UriOrString>,
    /// Claim names to require, in addition to the default-required `exp` claim.
    pub required_claims: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Tracing {
    pub level: String,
    /// The name of the service to use for the OpenTelemetry `service.name` field. If not provided,
    /// will use the [`App::name`][App] config value, translated to `snake_case`.
    pub service_name: Option<String>,
    /// Propagate traces across service boundaries. Mostly useful in microservice architectures.
    #[serde(default = "default_true")]
    pub trace_propagation: bool,
    /// URI of the OTLP exporter where traces/metics/logs will be sent.
    pub otlp_endpoint: Option<Url>,
}

#[cfg(feature = "db-sql")]
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

#[cfg(feature = "db-sql")]
impl Database {
    fn default_connect_timeout() -> Duration {
        Duration::from_millis(1000)
    }

    fn default_acquire_timeout() -> Duration {
        Duration::from_millis(1000)
    }
}

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Worker {
    // Todo: Make Redis optional for workers?
    #[cfg(feature = "sidekiq")]
    pub sidekiq: Sidekiq,
}

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Sidekiq {
    // Todo: Make Redis optional for workers?
    pub redis: Redis,
    #[serde(default)]
    pub queue_names: Vec<String>,
}

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Redis {
    pub uri: Url,
    #[serde(default = "Redis::default_min_idle")]
    pub min_idle: Option<u32>,
    #[serde(default = "Redis::default_max_connections")]
    pub max_connections: u32,
}

#[cfg(feature = "sidekiq")]
impl Redis {
    fn default_min_idle() -> Option<u32> {
        Some(5)
    }

    fn default_max_connections() -> u32 {
        (num_cpus::get() + 5) as u32
    }
}
