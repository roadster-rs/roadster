use crate::config::auth::Auth;
#[cfg(feature = "db-sql")]
use crate::config::database::Database;
#[cfg(feature = "email")]
use crate::config::email::Email;
use crate::config::environment::{Environment, ENVIRONMENT_ENV_VAR_NAME};
use crate::config::health_check::HealthCheck;
use crate::config::lifecycle::LifecycleHandler;
use crate::config::service::Service;
use crate::config::tracing::Tracing;
use crate::error::RoadsterResult;
use crate::util::serde::default_true;
use ::tracing::warn;
use config::builder::DefaultState;
use config::{Config, ConfigBuilder, FileFormat};
use convert_case::Case;
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use validator::Validate;

pub mod app_config;
pub mod auth;
#[cfg(feature = "db-sql")]
pub mod database;
#[cfg(feature = "email")]
pub mod email;
pub mod environment;
pub mod health_check;
pub mod lifecycle;
pub mod service;
pub mod tracing;

pub type CustomConfig = BTreeMap<String, Value>;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct AppConfig {
    pub environment: Environment,
    #[validate(nested)]
    pub app: App,
    #[validate(nested)]
    pub lifecycle_handler: LifecycleHandler,
    #[validate(nested)]
    pub health_check: HealthCheck,
    #[validate(nested)]
    pub service: Service,
    #[validate(nested)]
    pub auth: Auth,
    #[validate(nested)]
    pub tracing: Tracing,
    #[cfg(feature = "db-sql")]
    #[validate(nested)]
    pub database: Database,
    #[cfg(feature = "email")]
    #[validate(nested)]
    pub email: Email,
    /// Allows providing custom config values. Any configs that aren't pre-defined above
    /// will be collected here.
    ///
    /// # Examples
    ///
    /// ```toml
    /// [foo]
    /// x = "y"
    /// ```
    ///
    /// This will be parsed as:
    /// ```raw
    /// AppConfig#custom: {
    ///     "foo": {
    ///         "x": "y",
    ///     }
    /// }
    /// ```
    #[serde(flatten, default)]
    pub custom: CustomConfig,
}

pub const ENV_VAR_PREFIX: &str = "ROADSTER";
pub const ENV_VAR_SEPARATOR: &str = "__";

impl AppConfig {
    #[deprecated(
        since = "0.6.2",
        note = "This wasn't intended to be made public and may be removed in a future version."
    )]
    pub fn new(environment: Option<Environment>) -> RoadsterResult<Self> {
        Self::new_with_config_dir(environment, Some(PathBuf::from("config/")))
    }

    // This runs before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub(crate) fn new_with_config_dir(
        environment: Option<Environment>,
        config_dir: Option<PathBuf>,
    ) -> RoadsterResult<Self> {
        dotenv().ok();

        let environment = if let Some(environment) = environment {
            println!("Using environment from CLI args: {environment:?}");
            environment
        } else {
            Environment::new()?
        };
        let environment_str: &str = environment.clone().into();

        let config_root_dir = config_dir
            .unwrap_or_else(|| PathBuf::from("config/"))
            .canonicalize()?;

        println!("Loading configuration from directory {config_root_dir:?}");

        let config = Self::default_config(environment);
        let config = config_env_file("default", &config_root_dir, config);
        let config = config_env_dir("default", &config_root_dir, config)?;
        let config = config_env_file(environment_str, &config_root_dir, config);
        let config = config_env_dir(environment_str, &config_root_dir, config)?;
        let config = config
            .add_source(
                config::Environment::default()
                    .prefix(ENV_VAR_PREFIX)
                    .convert_case(Case::Kebab)
                    .separator(ENV_VAR_SEPARATOR),
            )
            .set_override(ENVIRONMENT_ENV_VAR_NAME, environment_str)?
            .build()?;
        let config: AppConfig = config.try_deserialize()?;

        Ok(config)
    }

    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn test(config_str: Option<&str>) -> RoadsterResult<Self> {
        let config = Self::default_config(Environment::Test)
            .add_source(config::File::from_str(
                config_str.unwrap_or(
                    r#"
                    environment = "test"

                    [app]
                    name = "Test"

                    [tracing]
                    level = "debug"

                    [database]
                    uri = "postgres://example:example@invalid_host:5432/example_test"
                    auto-migrate = true
                    max-connections = 10

                    [auth.jwt]
                    secret = "secret-test"

                    [service.http]
                    host = "127.0.0.1"
                    port = 3000

                    [service.grpc]
                    host = "127.0.0.1"
                    port = 3001

                    [service.sidekiq]
                    # This field normally is determined by the number of CPU cores if not provided.
                    # We provide it in the test config to avoid snapshot failures when running
                    # on varying hardware.
                    num-workers = 16

                    [service.sidekiq.redis]
                    uri = "redis://invalid_host:1234"

                    [email.from]
                    email = "no-reply@example.com"

                    [email.smtp.connection]
                    uri = "smtps://username:password@smtp.example.com:425"

                    [email.sendgrid]
                    api-key = "api-key"
                    "#,
                ),
                FileFormat::Toml,
            ))
            .build()?;

        let config: AppConfig = config.try_deserialize()?;
        Ok(config)
    }

    #[allow(clippy::let_and_return)]
    fn default_config(
        #[allow(unused_variables)] environment: Environment,
    ) -> ConfigBuilder<DefaultState> {
        let config = Config::builder()
            .add_source(config::File::from_str(
                include_str!("default.toml"),
                FileFormat::Toml,
            ))
            .add_source(crate::config::tracing::default_config());

        #[cfg(feature = "http")]
        let config = {
            let config = config.add_source(crate::config::service::http::default_config());
            let config = crate::config::service::http::default_config_per_env(environment)
                .into_iter()
                .fold(config, |config, source| config.add_source(source));
            config
        };

        #[cfg(feature = "grpc")]
        let config = config.add_source(crate::config::service::grpc::default_config());

        #[cfg(feature = "sidekiq")]
        let config = config.add_source(crate::config::service::worker::sidekiq::default_config());

        let config = config.add_source(crate::config::lifecycle::default_config());

        let config = config.add_source(crate::config::health_check::default_config());

        config
    }

    pub(crate) fn validate(&self, exit_on_error: bool) -> RoadsterResult<()> {
        let result = Validate::validate(self);
        if exit_on_error {
            result?;
        } else if let Err(err) = result {
            warn!("An error occurred when validating the app config: {}", err);
        }
        Ok(())
    }
}

/// Adds a config file in the relative path `config/{environment}.toml` to the
/// [`ConfigBuilder`]. If no such file exists, does nothing.
fn config_env_file(
    environment: &str,
    config_dir: &Path,
    config: ConfigBuilder<DefaultState>,
) -> ConfigBuilder<DefaultState> {
    // Todo: allow other file formats?
    let path = config_dir.join(format!("{environment}.toml"));
    if !path.is_file() {
        return config;
    }

    config.add_source(config::File::from(path))
}

/// Recursively adds all the config files in the given relative path `config/{environment}/` to the
/// [`ConfigBuilder`]. If no such directory exists, does nothing.
fn config_env_dir(
    environment: &str,
    config_dir: &Path,
    config: ConfigBuilder<DefaultState>,
) -> RoadsterResult<ConfigBuilder<DefaultState>> {
    let path = config_dir.join(environment);
    if !path.is_dir() {
        return Ok(config);
    }

    config_env_dir_recursive(&path, config)
}

/// Helper method for [`config_env_dir`] to recursively add config files in the given path
/// to the [`ConfigBuilder`].
// Todo: allow other file formats?
fn config_env_dir_recursive(
    path: &Path,
    config: ConfigBuilder<DefaultState>,
) -> RoadsterResult<ConfigBuilder<DefaultState>> {
    fs::read_dir(path)?.try_fold(config, |config, dir_entry| {
        let path = dir_entry?.path();
        if path.is_dir() {
            config_env_dir_recursive(&path, config)
        } else if path.is_file() && path.extension().unwrap_or_default() == "toml" {
            Ok(config.add_source(config::File::from(path)))
        } else {
            Ok(config)
        }
    })
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct App {
    pub name: String,
    /// Shutdown the whole app if an error occurs in one of the app's top-level tasks (API, workers, etc).
    #[serde(default = "default_true")]
    pub shutdown_on_error: bool,
}

#[cfg(all(
    test,
    feature = "http",
    feature = "grpc",
    feature = "sidekiq",
    feature = "db-sql",
    feature = "open-api",
    feature = "jwt",
    feature = "jwt-ietf",
    feature = "otel",
    feature = "email-smtp"
))]
mod tests {
    use super::*;
    use insta::assert_toml_snapshot;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn test() {
        let config = AppConfig::test(None).unwrap();

        assert_toml_snapshot!(config);
    }
}
