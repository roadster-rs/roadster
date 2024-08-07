use crate::config::auth::Auth;
#[cfg(feature = "db-sql")]
use crate::config::database::Database;
use crate::config::environment::{Environment, ENVIRONMENT_ENV_VAR_NAME};
use crate::config::health_check::HealthCheck;
use crate::config::service::Service;
use crate::config::tracing::Tracing;
use crate::error::RoadsterResult;
use crate::util::serde::default_true;
use config::builder::DefaultState;
use config::{Case, Config, ConfigBuilder, FileFormat};
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use tracing::warn;
use validator::Validate;

pub type CustomConfig = BTreeMap<String, Value>;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct AppConfig {
    pub environment: Environment,
    #[validate(nested)]
    pub app: App,
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
    // This runs before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub fn new(environment: Option<Environment>) -> RoadsterResult<Self> {
        dotenv().ok();

        let environment = if let Some(environment) = environment {
            println!("Using environment from CLI args: {environment:?}");
            environment
        } else {
            Environment::new()?
        };
        let environment_str: &str = environment.clone().into();

        let config = Self::default_config(environment)
            // Todo: allow other file formats?
            // Todo: allow splitting config into multiple files?
            .add_source(config::File::with_name("config/default.toml"))
            .add_source(config::File::with_name(&format!(
                "config/{environment_str}.toml"
            )))
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
    feature = "otel"
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
