#[cfg(feature = "cli")]
use crate::api::cli::parse_cli;
#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
use crate::app::App;
use crate::app::context::AppContext;
use crate::config::environment::Environment;
use crate::config::{AppConfig, AppConfigOptions, ConfigOverrideSource};
#[cfg(feature = "db-sql")]
use crate::db::migration::Migrator;
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::service::registry::ServiceRegistry;
use axum_core::extract::FromRef;
use std::marker::PhantomData;
use std::path::PathBuf;
use typed_builder::TypedBuilder;

/// Contains all the objects needed to run the [`App`]. Useful if a consumer needs access to some
/// of the prepared state before running the app.
///
/// Created by [`prepare`]. Pass to [`crate::app::run_prepared`] to run the [`App`].
#[non_exhaustive]
pub struct PreparedApp<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    pub cli: Option<PreparedAppCli<A, S>>,
    pub app: A,
    pub state: S,
    #[cfg(feature = "db-sql")]
    pub migrators: Vec<Box<dyn Migrator<S>>>,
    pub service_registry: ServiceRegistry<A, S>,
    pub lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

#[non_exhaustive]
pub struct PreparedAppCli<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    pub roadster_cli: RoadsterCli,
    #[cfg(feature = "cli")]
    pub app_cli: A::Cli,
    pub(crate) _app: PhantomData<A>,
    pub(crate) _state: PhantomData<S>,
}

/// Options to use when preparing the app. Normally these values can be provided via env vars
/// or CLI arguments when running the [`crate::app::run`] method. However, if [`prepare`] is called
/// directly, especially from somewhere without an env or CLI, then this can be used to configure
/// the prepared app.
#[derive(Default, Debug, TypedBuilder)]
#[non_exhaustive]
#[builder(mutators(
    fn config_sources(&mut self, config_sources: Vec<Box<dyn config::Source + Send + Sync>>) -> &mut Self{
        self.config_sources = config_sources;
    self
    }
    pub fn add_config_source(&mut self, source: impl config::Source + Send + Sync + 'static) -> &mut Self{
        self.config_sources.push(Box::new(source));
    self
    }
    pub fn add_config_source_boxed(&mut self, source: Box<dyn config::Source + Send + Sync>) -> &mut Self{
        self.config_sources.push(source);
    self
    }
))]
pub struct PrepareOptions {
    #[builder(default, setter(strip_option))]
    pub env: Option<Environment>,

    #[builder(default = true)]
    pub parse_cli: bool,

    #[builder(default, setter(strip_option))]
    pub config_dir: Option<PathBuf>,

    /// Manually provide custom config sources. This is mostly intended to allow overriding
    /// specific app config fields for tests (e.g., using the [`ConfigOverrideSource`]), but it
    /// can also be used to provide other custom config sources outside of tests.
    #[builder(via_mutators)]
    pub config_sources: Vec<Box<dyn config::Source + Send + Sync>>,

    /// Explicitly override the entire [`AppConfig`] to run the app with. If provided, the other
    /// config-related fields in this struct will not be used.
    #[builder(default, setter(strip_option))]
    pub config: Option<AppConfig>,
}

impl PrepareOptions {
    /// The default recommended [`PrepareOptions`] to use in tests.
    pub fn test() -> Self {
        PrepareOptions::builder()
            .env(Environment::Test)
            .parse_cli(false)
            .build()
    }

    /// Provide an override for a specific config field.
    pub fn with_config_override(mut self, name: String, value: config::Value) -> Self {
        self.config_sources.push(Box::new(
            ConfigOverrideSource::builder()
                .name(name)
                .value(value)
                .build(),
        ));
        self
    }

    /// Override the entire [`AppConfig`].
    pub fn with_config(mut self, config: AppConfig) -> Self {
        self.config = Some(config);
        self
    }
}

/// Prepare the app. Sets up everything needed to start the app, but does not execute anything.
/// Specifically, the following are skipped:
///
/// 1. Handling CLI commands
/// 2. Health checks
/// 3. Lifecycle Handlers
/// 4. Starting any services
pub async fn prepare<A, S>(app: A, options: PrepareOptions) -> RoadsterResult<PreparedApp<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    prepare_from_cli_and_state(build_cli_and_state(app, options).await?).await
}

// This runs before tracing is initialized, so we need to use `println` in order to
// log from this method.
#[allow(clippy::disallowed_macros)]
pub(crate) async fn build_cli_and_state<A, S>(
    app: A,
    options: PrepareOptions,
) -> RoadsterResult<CliAndState<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = if options.parse_cli {
        let (roadster_cli, app_cli) = parse_cli::<A, S, _, _>(std::env::args_os())?;
        (Some(roadster_cli), Some(app_cli))
    } else {
        (None, None)
    };

    #[cfg(feature = "cli")]
    let environment = roadster_cli
        .as_ref()
        .and_then(|cli| cli.environment.clone())
        .or(options.env);
    #[cfg(not(feature = "cli"))]
    let environment: Option<Environment> = options.env;

    let environment = if let Some(environment) = environment {
        println!("Using environment: {environment:?}");
        environment
    } else {
        Environment::new()?
    };

    #[cfg(feature = "cli")]
    let config_dir = roadster_cli
        .as_ref()
        .and_then(|cli| cli.config_dir.clone())
        .or(options.config_dir);
    #[cfg(not(feature = "cli"))]
    let config_dir: Option<std::path::PathBuf> = options.config_dir;

    let async_config_sources = app.async_config_sources(&environment)?;

    let app_config_options = AppConfigOptions::builder()
        .environment(environment)
        .config_dir_opt(config_dir)
        .config_sources(options.config_sources);
    let app_config_options = async_config_sources
        .into_iter()
        .fold(app_config_options, |app_config_options, source| {
            app_config_options.add_async_source_boxed(source)
        })
        .build();
    let config = if let Some(config) = options.config {
        config
    } else {
        AppConfig::new_with_options(app_config_options).await?
    };

    app.init_tracing(&config)?;

    #[cfg(not(feature = "cli"))]
    config.validate(true)?;
    #[cfg(feature = "cli")]
    config.validate(
        !roadster_cli
            .as_ref()
            .map(|cli| cli.skip_validate_config)
            .unwrap_or_default(),
    )?;

    let state = build_state(&app, config).await?;

    Ok(CliAndState {
        app,
        #[cfg(feature = "cli")]
        roadster_cli,
        #[cfg(feature = "cli")]
        app_cli,
        state,
    })
}

/// Utility method to build the app's state object.
pub(crate) async fn build_state<A, S>(app: &A, config: AppConfig) -> RoadsterResult<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    #[cfg(not(test))]
    let metadata = app.metadata(&config)?;

    let mut extension_registry = Default::default();
    app.provide_context_extensions(&config, &mut extension_registry)
        .await?;

    // The `config.clone()` here is technically not necessary. However, without it, RustRover
    // is giving a "value used after move" error when creating an actual `AppContext` below.
    #[cfg(test)]
    let context = AppContext::test(Some(config.clone()), None, None)?;
    #[cfg(not(test))]
    let context = AppContext::new::<A, S>(app, config, metadata, extension_registry).await?;

    app.provide_state(context).await
}

pub(crate) async fn prepare_from_cli_and_state<A, S>(
    cli_and_state: CliAndState<A, S>,
) -> RoadsterResult<PreparedApp<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let CliAndState {
        app,
        #[cfg(feature = "cli")]
        roadster_cli,
        #[cfg(feature = "cli")]
        app_cli,
        state,
    } = cli_and_state;

    let PreparedAppWithoutCli {
        app,
        state,
        #[cfg(feature = "db-sql")]
        migrators,
        service_registry,
        lifecycle_handler_registry,
    } = prepare_without_cli(app, state).await?;

    #[cfg(feature = "cli")]
    let cli = if let Some((roadster_cli, app_cli)) = roadster_cli.zip(app_cli) {
        Some(PreparedAppCli {
            roadster_cli,
            app_cli,
            _app: Default::default(),
            _state: Default::default(),
        })
    } else {
        None
    };

    Ok(PreparedApp {
        #[cfg(feature = "cli")]
        cli,
        app,
        #[cfg(feature = "db-sql")]
        migrators,
        state,
        service_registry,
        lifecycle_handler_registry,
    })
}

#[non_exhaustive]
pub struct PreparedAppWithoutCli<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub app: A,
    pub state: S,
    #[cfg(feature = "db-sql")]
    pub migrators: Vec<Box<dyn Migrator<S>>>,
    pub service_registry: ServiceRegistry<A, S>,
    pub lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

pub(crate) async fn prepare_without_cli<A, S>(
    app: A,
    state: S,
) -> RoadsterResult<PreparedAppWithoutCli<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let context = AppContext::from_ref(&state);

    #[cfg(feature = "db-sql")]
    let migrators = app.migrators(&state)?;

    let mut lifecycle_handler_registry = LifecycleHandlerRegistry::new(&state);
    app.lifecycle_handlers(&mut lifecycle_handler_registry, &state)
        .await?;

    let mut health_check_registry = HealthCheckRegistry::new(&context);
    app.health_checks(&mut health_check_registry, &state)
        .await?;
    context.set_health_checks(health_check_registry)?;

    let mut service_registry = ServiceRegistry::new(&state);
    app.services(&mut service_registry, &state).await?;

    Ok(PreparedAppWithoutCli {
        app,
        state,
        #[cfg(feature = "db-sql")]
        migrators,
        service_registry,
        lifecycle_handler_registry,
    })
}

#[non_exhaustive]
pub(crate) struct CliAndState<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub app: A,
    #[cfg(feature = "cli")]
    pub roadster_cli: Option<RoadsterCli>,
    #[cfg(feature = "cli")]
    pub app_cli: Option<A::Cli>,
    pub state: S,
}

#[cfg(test)]
mod tests {
    use crate::app::prepare::PrepareOptions;
    use insta::assert_debug_snapshot;

    #[test]
    fn prepare_options_test() {
        let options = PrepareOptions::test();
        assert_debug_snapshot!(options);
    }
}
