use crate::config::auth::Auth;
#[cfg(feature = "db-sql")]
use crate::config::database::Database;
#[cfg(feature = "email")]
use crate::config::email::Email;
use crate::config::environment::{ENVIRONMENT_ENV_VAR_NAME, Environment};
use crate::config::lifecycle::LifecycleHandler;
use crate::config::service::Service;
#[cfg(feature = "testing")]
use crate::config::testing::Testing;
use crate::config::tracing::Tracing;
use crate::error::RoadsterResult;
use crate::util::serde::default_true;
use ::tracing::warn;
use cfg_if::cfg_if;
use config::builder::DefaultState;
use config::{AsyncSource, Config, ConfigBuilder, FileFormat, Map, Source};
use convert_case::Casing;
use dotenvy::dotenv;
use health::check;
use health::check::HealthCheck;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use validator::{Validate, ValidationErrors};

pub mod auth;
#[cfg(feature = "db-sql")]
pub mod database;
#[cfg(feature = "email")]
pub mod email;
pub mod environment;
pub mod health;
pub mod lifecycle;
pub mod service;
#[cfg(feature = "testing")]
pub mod testing;
pub mod tracing;

#[serde_with::skip_serializing_none]
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
    #[cfg(feature = "testing")]
    #[validate(nested)]
    pub testing: Testing,
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
    #[validate(nested)]
    pub custom: CustomConfig,
}

#[serde_with::skip_serializing_none]
#[derive(
    Debug,
    Default,
    Clone,
    derive_more::Deref,
    derive_more::DerefMut,
    Validate,
    Serialize,
    Deserialize,
)]
pub struct CustomConfig {
    #[serde(flatten)]
    inner: BTreeMap<String, Value>,
}

impl From<CustomConfig> for BTreeMap<String, Value> {
    fn from(value: CustomConfig) -> Self {
        value.inner
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyConfig;
impl Validate for EmptyConfig {
    fn validate(&self) -> Result<(), ValidationErrors> {
        Ok(())
    }
}

pub const ENV_VAR_PREFIX: &str = "ROADSTER";
pub const ENV_VAR_SEPARATOR: &str = "__";

const DEFAULT_CONFIG_DIR: &str = "config/";

// Note that config files are loaded in the provided order, and files loaded later will override
// any duplicate values from files loaded earlier. So, basically, the last extension has a higher
// priority than the first extension.
cfg_if! {
    if #[cfg(feature = "config-yml")] {
        pub const FILE_EXTENSIONS: [&str; 3] = ["yml", "yaml", "toml"];
    } else {
        pub const FILE_EXTENSIONS: [&str; 1] = ["toml"];
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, bon::Builder)]
pub struct ConfigOverrideSource {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub value: config::Value,
}

impl Source for ConfigOverrideSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<Map<String, config::Value>, config::ConfigError> {
        Ok([(self.name.clone(), self.value.clone())]
            .into_iter()
            .collect())
    }
}

#[derive(bon::Builder)]
#[non_exhaustive]
pub struct AppConfigOptions {
    #[builder(field)]
    pub config_sources: Vec<Box<dyn Source + Send + Sync>>,
    #[builder(field)]
    pub async_config_sources: Vec<Box<dyn AsyncSource + Send>>,
    pub environment: Environment,
    #[builder(into)]
    pub config_dir: Option<PathBuf>,
}

impl<S: app_config_options_builder::State> AppConfigOptionsBuilder<S> {
    pub(crate) fn config_sources(
        mut self,
        config_sources: Vec<Box<dyn Source + Send + Sync>>,
    ) -> Self {
        self.config_sources = config_sources;
        self
    }

    pub fn add_source(mut self, source: impl Source + Send + Sync + 'static) -> Self {
        self.config_sources.push(Box::new(source));
        self
    }

    pub fn add_source_boxed(mut self, source: Box<dyn Source + Send + Sync>) -> Self {
        self.config_sources.push(source);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn async_config_sources(
        mut self,
        async_config_sources: Vec<Box<dyn AsyncSource + Send>>,
    ) -> Self {
        self.async_config_sources = async_config_sources;
        self
    }

    pub fn add_async_source(mut self, source: impl AsyncSource + Send + 'static) -> Self {
        self.async_config_sources.push(Box::new(source));
        self
    }

    pub fn add_async_source_boxed(mut self, source: Box<dyn AsyncSource + Send>) -> Self {
        self.async_config_sources.push(source);
        self
    }
}

impl AppConfig {
    // This runs before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub async fn new_with_options(options: AppConfigOptions) -> RoadsterResult<Self> {
        dotenv().ok();

        let environment_string = options.environment.clone().to_string();
        let environment_str = environment_string.as_str();

        let config_root_dir = options
            .config_dir
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_DIR))
            .canonicalize()?;

        println!("Loading configuration from directory {config_root_dir:?}");

        let config = Self::default_config(options.environment.clone())?;
        let config = config_env_file("default", &config_root_dir, config);
        let config = config_env_dir("default", &config_root_dir, config)?;
        let config = config_env_file(environment_str, &config_root_dir, config);
        let config = config_env_dir(environment_str, &config_root_dir, config)?;
        let config = config.add_source(
            config::Environment::default()
                .prefix(ENV_VAR_PREFIX)
                .convert_case(config::Case::Kebab)
                .separator(ENV_VAR_SEPARATOR),
        );

        // Convert builder state to `AsyncState`
        let config = config.add_async_source(BoxedAsyncSource(None));

        // Add all of the provided non-async sources
        let config = options
            .config_sources
            .into_iter()
            .fold(config, |config, source| {
                config.add_source(BoxedSource(source))
            });

        // Add all of the provided async sources
        let config = options
            .async_config_sources
            .into_iter()
            .fold(config, |config, source| {
                config.add_async_source(BoxedAsyncSource(Some(source)))
            });

        let config = config
            .set_override(
                ENVIRONMENT_ENV_VAR_NAME.to_case(convert_case::Case::Kebab),
                environment_str,
            )?
            .build()
            .await?;
        let config: AppConfig = config.try_deserialize()?;

        Ok(config)
    }

    #[cfg(test)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn test(config_str: Option<&str>) -> RoadsterResult<Self> {
        let config = Self::default_config(Environment::Test)?
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
                    scheme = "http"
                    host = "127.0.0.1"
                    port = 3000

                    [service.grpc]
                    scheme = "http"
                    host = "127.0.0.1"
                    port = 3001

                    [service.worker.sidekiq]
                    # This field normally is determined by the number of CPU cores if not provided.
                    # We provide it in the test config to avoid snapshot failures when running
                    # on varying hardware.
                    num-workers = 16

                    [service.worker.sidekiq.redis]
                    uri = "redis://invalid_host:1234"

                    [service.worker.pg]
                    # This field normally is determined by the number of CPU cores if not provided.
                    # We provide it in the test config to avoid snapshot failures when running
                    # on varying hardware.
                    num-workers = 16

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
    ) -> RoadsterResult<ConfigBuilder<DefaultState>> {
        let config = Config::builder()
            .set_default("environment", environment.clone().to_string())?
            .add_source(config::File::from_str(
                include_str!("default.toml"),
                FileFormat::Toml,
            ));

        let config = {
            let config = config.add_source(tracing::default_config());
            let config = tracing::default_config_per_env(environment.clone())
                .into_iter()
                .fold(config, |config, source| config.add_source(source));
            config
        };

        #[cfg(feature = "http")]
        let config = {
            let config = config.add_source(service::http::default_config());
            let config = service::http::default_config_per_env(environment.clone())
                .into_iter()
                .fold(config, |config, source| config.add_source(source));
            config
        };

        #[cfg(feature = "grpc")]
        let config = {
            let config = config.add_source(service::grpc::default_config());
            let config = service::grpc::default_config_per_env(environment.clone())
                .into_iter()
                .fold(config, |config, source| config.add_source(source));
            config
        };

        #[cfg(feature = "worker")]
        let config = config.add_source(service::worker::default_config());

        let config = config.add_source(lifecycle::default_config());

        let config = config.add_source(check::default_config());

        #[cfg(feature = "email-sendgrid")]
        let config = {
            let config = email::sendgrid::default_config_per_env(environment)
                .into_iter()
                .fold(config, |config, source| config.add_source(source));
            config
        };

        #[cfg(feature = "testing")]
        let config = config.add_source(testing::default_config());

        Ok(config)
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

/// Adds the config file from the given config_dir for the given environment. The config file
/// can have any file extension specified in [`FILE_EXTENSIONS`].
fn config_env_file(
    environment: &str,
    config_dir: &Path,
    config: ConfigBuilder<DefaultState>,
) -> ConfigBuilder<DefaultState> {
    FILE_EXTENSIONS
        .map(|ext| config_dir.join(format!("{environment}.{ext}")))
        .into_iter()
        .filter(|path| path.is_file())
        .fold(config, |config, path| {
            config.add_source(config::File::from(path))
        })
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
fn config_env_dir_recursive(
    path: &Path,
    config: ConfigBuilder<DefaultState>,
) -> RoadsterResult<ConfigBuilder<DefaultState>> {
    fs::read_dir(path)?.try_fold(config, |config, dir_entry| {
        let path = dir_entry?.path();
        if path.is_dir() {
            config_env_dir_recursive(&path, config)
        } else if path.is_file()
            && FILE_EXTENSIONS
                .iter()
                .any(|ext| *ext == path.extension().unwrap_or_default())
        {
            Ok(config.add_source(config::File::from(path)))
        } else {
            Ok(config)
        }
    })
}

#[derive(Debug)]
struct BoxedSource(Box<dyn Source + Send + Sync>);

impl Source for BoxedSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        self.0.clone_into_box()
    }

    fn collect(&self) -> Result<Map<String, config::Value>, config::ConfigError> {
        self.0.collect()
    }
}

#[derive(Debug)]
struct BoxedAsyncSource(Option<Box<dyn AsyncSource + Send + Sync>>);

#[async_trait::async_trait]
impl AsyncSource for BoxedAsyncSource {
    async fn collect(&self) -> Result<Map<String, config::Value>, config::ConfigError> {
        if let Some(source) = self.0.as_ref() {
            source.collect().await
        } else {
            Ok(Default::default())
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct App {
    pub name: String,
    /// Shutdown the whole app if an error occurs in one of the app's top-level tasks (API, workers, etc).
    #[serde(default = "default_true")]
    pub shutdown_on_error: bool,
}

#[cfg(feature = "test-containers")]
#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct TestContainer {
    pub enable: bool,
    pub tag: String,
}

#[cfg(all(
    test,
    feature = "default",
    feature = "default-diesel",
    feature = "open-api",
    feature = "worker-sidekiq",
    feature = "worker-pg",
    feature = "db-sea-orm",
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool",
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async",
    feature = "email-smtp",
    feature = "email-sendgrid",
    feature = "jwt-ietf",
    feature = "jwt-openid",
    feature = "cli",
    feature = "otel",
    feature = "grpc",
    feature = "test-containers",
    feature = "testing-mocks",
    feature = "config-yml",
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

#[cfg(test)]
mod custom_config_tests {
    use crate::config::CustomConfig;
    use serde_json::Value;
    use std::collections::BTreeMap;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_map() {
        let config: CustomConfig = CustomConfig {
            inner: BTreeMap::new(),
        };
        let _map: BTreeMap<String, Value> = config.into();
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deref() {
        let mut inner = BTreeMap::new();
        inner.insert("foo".to_string(), "bar".into());
        let config: CustomConfig = CustomConfig { inner };
        assert_eq!(config.get("foo").unwrap(), "bar");
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn build() {
        let mut config = CustomConfig::default();
        config.insert("key".to_owned(), Value::String("value".to_owned()));
    }
}

#[cfg(test)]
mod file_extensions_tests {
    use insta::assert_debug_snapshot;

    #[test]
    #[cfg(not(feature = "config-yml"))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn file_extensions_no_yml() {
        assert_debug_snapshot!(super::FILE_EXTENSIONS);
    }

    #[test]
    #[cfg(feature = "config-yml")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn file_extensions_yml() {
        assert_debug_snapshot!(super::FILE_EXTENSIONS);
    }
}

#[cfg(test)]
mod app_config_options_tests {
    use crate::config::AppConfigOptions;
    use crate::config::environment::Environment;
    use config::{AsyncSource, Map, Value};

    #[derive(Debug)]
    struct TestAsyncSource;

    #[async_trait::async_trait]
    impl AsyncSource for TestAsyncSource {
        async fn collect(&self) -> Result<Map<String, Value>, config::ConfigError> {
            Ok(Default::default())
        }
    }

    #[test]
    fn app_config_options_builder() {
        let builder = AppConfigOptions::builder()
            .environment(Environment::Test)
            .config_dir("./")
            .async_config_sources(vec![Box::new(TestAsyncSource)])
            .add_async_source(TestAsyncSource)
            .add_async_source_boxed(Box::new(TestAsyncSource));

        let options = builder.build();

        assert_eq!(options.async_config_sources.len(), 3);
    }
}
