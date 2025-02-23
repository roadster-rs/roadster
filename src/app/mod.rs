pub mod context;
pub mod metadata;
mod roadster_app;

/// A default implementation of [`App`] that is customizable via a builder-style API.
///
/// See <https://github.com/roadster-rs/roadster/tree/main/examples/app-builder/src/main.rs> for
/// an example of how to use the [`RoadsterApp`].
///
/// The `Cli` type parameter is only required when the using a custom CLI.
pub use roadster_app::RoadsterApp;

/// Builder-style API to build/customize a [`RoadsterApp`].
///
/// See <https://github.com/roadster-rs/roadster/tree/main/examples/app-builder/src/main.rs> for
/// an example of how to use the [`RoadsterAppBuilder`].
///
/// The `Cli` type parameter is only required when the using a custom CLI.
pub use roadster_app::RoadsterAppBuilder;

#[cfg(feature = "cli")]
use crate::api::cli::parse_cli;
#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
#[cfg(all(test, feature = "cli"))]
use crate::api::cli::MockTestCli;
#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::metadata::AppMetadata;
use crate::config::environment::Environment;
use crate::config::{AppConfig, AppConfigOptions};
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
#[cfg(feature = "db-sql")]
use crate::migration::Migrator;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use context::AppContext;
#[cfg(feature = "db-sea-orm")]
use sea_orm::ConnectOptions;
#[cfg(feature = "cli")]
use std::env;
use std::future;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};
use typed_builder::TypedBuilder;

/// Run the [`App`]
pub async fn run<A, S>(app: A) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let cli_and_state = build_cli_and_state(app, PrepareOptions::builder().build()).await?;

    let prepared = prepare_from_cli_and_state(cli_and_state).await?;

    #[cfg(feature = "cli")]
    if crate::api::cli::handle_cli(&prepared).await? {
        return Ok(());
    }

    run_prepared_without_cli(prepared).await
}

/// Similar to [`run`], except intended to be used in tests. Does all of the same setup and
/// teardown logic as [`run`], but does not actually run the registered
/// [`crate::service::AppService`]s.
///
/// Note: If the test panics, the teardown logic will not be run.
#[cfg(feature = "testing")]
pub async fn run_test<A, S>(
    app: A,
    options: PrepareOptions,
    // todo: RustRover doesn't seem to recognize `AsyncFn`. Does it just need an update?
    test_fn: impl std::ops::AsyncFn(&PreRunAppState<A, S>),
) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let prepared = prepare(app, options).await?;

    before_app(&prepared).await?;

    let pre_run_app_state = PreRunAppState {
        app: prepared.app,
        state: prepared.state.clone(),
        service_registry: prepared.service_registry,
    };

    tracing::debug!("Starting test");

    test_fn(&pre_run_app_state).await;

    tracing::debug!("Test complete");

    after_app(&prepared.lifecycle_handler_registry, &prepared.state).await?;

    Ok(())
}

/// Similar to [`run_test`], except allows returning a [`Result`] to communicate test
/// success/failure. If the test returns an [`Err`], the teardown logic will still be run. If the
/// test returns an [`Err`], it will then be returned in the [`Err`] returned by
/// [`run_test_with_result`] itself.
///
/// Note: If the test panics, the teardown logic will not be run. To ensure the teardown logic runs,
/// return an error instead of panicking.
#[cfg(feature = "testing")]
pub async fn run_test_with_result<A, S, T, E>(
    app: A,
    options: PrepareOptions,
    // todo: RustRover doesn't seem to recognize `AsyncFn`. Does it just need an update?
    test_fn: T,
) -> Result<(), (Option<crate::error::Error>, Option<E>)>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
    T: std::ops::AsyncFn(&PreRunAppState<A, S>) -> Result<(), E>,
    E: std::error::Error,
{
    let prepared = match prepare(app, options).await {
        Ok(prepared) => prepared,
        Err(err) => return Err((Some(err), None)),
    };

    if let Err(err) = before_app(&prepared).await {
        return Err((Some(err), None));
    }

    let pre_run_app_state = PreRunAppState {
        app: prepared.app,
        state: prepared.state.clone(),
        service_registry: prepared.service_registry,
    };

    tracing::debug!("Starting test");

    let test_result = test_fn(&pre_run_app_state).await;

    tracing::debug!("Test complete");

    let after_app_result = after_app(&prepared.lifecycle_handler_registry, &prepared.state).await;

    let after_app_result = if let Err(err) = after_app_result {
        Some(err)
    } else {
        None
    };

    let test_result = if let Err(err) = test_result {
        Some(err)
    } else {
        None
    };

    if after_app_result.is_some() || test_result.is_some() {
        return Err((after_app_result, test_result));
    }

    Ok(())
}

#[non_exhaustive]
struct CliAndState<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub app: A,
    #[cfg(feature = "cli")]
    pub roadster_cli: RoadsterCli,
    #[cfg(feature = "cli")]
    pub app_cli: A::Cli,
    pub state: S,
}

// This runs before tracing is initialized, so we need to use `println` in order to
// log from this method.
#[allow(clippy::disallowed_macros)]
async fn build_cli_and_state<A, S>(
    app: A,
    options: PrepareOptions,
) -> RoadsterResult<CliAndState<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = parse_cli::<A, S, _, _>(env::args_os())?;

    #[cfg(feature = "cli")]
    let environment = roadster_cli.environment.clone().or(options.env);
    #[cfg(not(feature = "cli"))]
    let environment: Option<Environment> = options.env;

    let environment = if let Some(environment) = environment {
        println!("Using environment: {environment:?}");
        environment
    } else {
        Environment::new()?
    };

    #[cfg(feature = "cli")]
    let config_dir = roadster_cli.config_dir.clone().or(options.config_dir);
    #[cfg(not(feature = "cli"))]
    let config_dir: Option<std::path::PathBuf> = options.config_dir;

    let async_config_sources = app.async_config_sources(&environment)?;

    let app_config_options = AppConfigOptions::builder()
        .environment(environment)
        .config_dir_opt(config_dir);
    let app_config_options = async_config_sources
        .into_iter()
        .fold(app_config_options, |app_config_options, source| {
            app_config_options.add_async_source_boxed(source)
        })
        .build();
    let config = AppConfig::new_with_options(app_config_options).await?;

    app.init_tracing(&config)?;

    #[cfg(not(feature = "cli"))]
    config.validate(true)?;
    #[cfg(feature = "cli")]
    config.validate(!roadster_cli.skip_validate_config)?;

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
async fn build_state<A, S>(app: &A, config: AppConfig) -> RoadsterResult<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    #[cfg(not(test))]
    let metadata = app.metadata(&config)?;

    // The `config.clone()` here is technically not necessary. However, without it, RustRover
    // is giving a "value used after move" error when creating an actual `AppContext` below.
    #[cfg(test)]
    let context = AppContext::test(Some(config.clone()), None, None)?;
    #[cfg(not(test))]
    let context = AppContext::new::<A, S>(app, config, metadata).await?;

    app.provide_state(context).await
}

/// Contains all the objects needed to run the [`App`]. Useful if a consumer needs access to some
/// of the prepared state before running the app.
///
/// Created by [`prepare`]. Pass to [`run_prepared`] to run the [`App`].
#[non_exhaustive]
pub struct PreparedApp<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    pub roadster_cli: RoadsterCli,
    #[cfg(feature = "cli")]
    pub app_cli: A::Cli,
    pub app: A,
    pub state: S,
    #[cfg(feature = "db-sql")]
    pub migrators: Vec<Box<dyn Migrator<S>>>,
    pub service_registry: ServiceRegistry<A, S>,
    pub lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

#[non_exhaustive]
pub struct PreRunAppState<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub app: A,
    pub state: S,
    pub service_registry: ServiceRegistry<A, S>,
}

/// Options to use when preparing the app. Normally these values can be provided via env vars
/// or CLI arguments when running the [`run`] method. However, if [`prepare`] is called directly,
/// especially from somewhere without an env or CLI, then this can be used to configure the
/// prepared app.
#[derive(Default, Debug, TypedBuilder)]
#[non_exhaustive]
pub struct PrepareOptions {
    #[builder(default, setter(strip_option))]
    pub env: Option<Environment>,
    #[builder(default, setter(strip_option))]
    pub config_dir: Option<PathBuf>,
}

impl PrepareOptions {
    pub fn test() -> Self {
        PrepareOptions::builder().env(Environment::Test).build()
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

/// Initialize the app state. Does everything needed to initialize the app state, but does not
/// run any other start up logic, such as running health checks, lifecycle handlers, or services.
///
/// This is intended to only be used to get access to the app's fully set up state in tests.
///
/// This is useful compared to [`run_test`] and [`run_test_with_result`] if you just need
/// access to your app's state and you don't need to run all of your app's startup/teardown logic
/// in your test.
#[cfg(feature = "testing")]
pub async fn test_state<A, S>(app: A, config: AppConfig) -> RoadsterResult<S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let state = build_state(&app, config).await?;

    let prepared_without_cli = prepare_without_cli(app, state).await?;
    let context = AppContext::from_ref(&prepared_without_cli.state);
    context.set_health_checks(prepared_without_cli.health_check_registry)?;

    Ok(prepared_without_cli.state)
}

async fn prepare_from_cli_and_state<A, S>(
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
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
    } = prepare_without_cli(app, state).await?;

    let context = AppContext::from_ref(&state);
    context.set_health_checks(health_check_registry)?;

    Ok(PreparedApp {
        app,
        #[cfg(feature = "cli")]
        roadster_cli,
        #[cfg(feature = "cli")]
        app_cli,
        #[cfg(feature = "db-sql")]
        migrators,
        state,
        service_registry,
        lifecycle_handler_registry,
    })
}

#[non_exhaustive]
struct PreparedAppWithoutCli<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    app: A,
    state: S,
    #[cfg(feature = "db-sql")]
    migrators: Vec<Box<dyn Migrator<S>>>,
    health_check_registry: HealthCheckRegistry,
    service_registry: ServiceRegistry<A, S>,
    lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

async fn prepare_without_cli<A, S>(app: A, state: S) -> RoadsterResult<PreparedAppWithoutCli<A, S>>
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

    let mut service_registry = ServiceRegistry::new(&state);
    app.services(&mut service_registry, &state).await?;

    Ok(PreparedAppWithoutCli {
        app,
        state,
        #[cfg(feature = "db-sql")]
        migrators,
        service_registry,
        lifecycle_handler_registry,
        health_check_registry,
    })
}

/// Run a [PreparedApp] that was previously crated by [prepare]
pub async fn run_prepared<A, S>(prepared_app: PreparedApp<A, S>) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    {
        if crate::api::cli::handle_cli(&prepared_app).await? {
            return Ok(());
        }
    }

    run_prepared_without_cli(prepared_app).await
}

/// Run the app's initialization logic (lifecycle handlers, health checks, etc).
async fn before_app<A, S>(prepared_app: &PreparedApp<A, S>) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    if prepared_app.service_registry.services.is_empty() {
        warn!("No enabled services were registered.");
    }

    let lifecycle_handlers = prepared_app
        .lifecycle_handler_registry
        .handlers(&prepared_app.state);

    info!("Running AppLifecycleHandler::before_health_checks");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_health_checks");
        handler.before_health_checks(prepared_app).await?;
    }

    let context = AppContext::from_ref(&prepared_app.state);
    crate::service::runner::health_checks(context.health_checks()).await?;

    info!("Running AppLifecycleHandler::before_services");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_services");
        handler.before_services(prepared_app).await?
    }
    crate::service::runner::before_run(&prepared_app.service_registry, &prepared_app.state).await?;

    Ok(())
}

/// Run the app's teardown logic.
async fn after_app<A, S>(
    lifecycle_handler_registry: &LifecycleHandlerRegistry<A, S>,
    state: &S,
) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    info!("Shutting down");

    let lifecycle_handlers = lifecycle_handler_registry.handlers(state);

    info!("Running AppLifecycleHandler::before_shutdown");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_shutdown");
        let result = handler.on_shutdown(state).await;
        if let Err(err) = result {
            error!(name=%handler.name(), "An error occurred when running AppLifecycleHandler::before_shutdown: {err}");
        }
    }

    info!("Shutdown complete");

    Ok(())
}

/// Run a [`PreparedApp`] that was previously crated by [`prepare`] without handling CLI commands
/// (they should have been handled already).
async fn run_prepared_without_cli<A, S>(prepared_app: PreparedApp<A, S>) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    before_app(&prepared_app).await?;

    let result = crate::service::runner::run(
        prepared_app.app,
        prepared_app.service_registry,
        &prepared_app.state,
    )
    .await;
    if let Err(err) = result {
        error!("An error occurred in the app: {err}");
    }

    after_app(
        &prepared_app.lifecycle_handler_registry,
        &prepared_app.state,
    )
    .await?;

    Ok(())
}

#[cfg_attr(all(test, feature = "cli"), mockall::automock(type Cli = MockTestCli<S>;))]
#[cfg_attr(all(test, not(feature = "cli")), mockall::automock(type Cli = crate::util::empty::Empty;))]
#[async_trait]
pub trait App<S>: Send + Sync + Sized
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    type Cli: clap::Args + RunCommand<Self, S> + Send + Sync;
    #[cfg(not(feature = "cli"))]
    type Cli;

    fn async_config_sources(
        &self,
        _environment: &Environment,
    ) -> RoadsterResult<Vec<Box<dyn config::AsyncSource + Send + Sync>>> {
        Ok(vec![])
    }

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        init_tracing(config, &self.metadata(config)?)?;

        Ok(())
    }

    fn metadata(&self, _config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(Default::default())
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        Ok(ConnectOptions::from(&config.database))
    }

    /// Provide the app state that will be used throughout the app. The state can simply be the
    /// provided [`AppContext`], or a custom type that implements [`FromRef`] to allow Roadster to
    /// extract its [`AppContext`] when needed.
    ///
    /// See the following for more details regarding [`FromRef`]: <https://docs.rs/axum/0.7.5/axum/extract/trait.FromRef.html>
    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S>;

    #[cfg(feature = "db-sql")]
    fn migrators(&self, _state: &S) -> RoadsterResult<Vec<Box<dyn Migrator<S>>>> {
        Ok(Default::default())
    }

    async fn lifecycle_handlers(
        &self,
        _registry: &mut LifecycleHandlerRegistry<Self, S>,
        _state: &S,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// Provide the [crate::health::check::HealthCheck]s to use throughout the app.
    async fn health_checks(
        &self,
        _registry: &mut HealthCheckRegistry,
        _state: &S,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// Provide the [crate::service::AppService]s to run in the app.
    async fn services(
        &self,
        _registry: &mut ServiceRegistry<Self, S>,
        _state: &S,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// Override to provide a custom shutdown signal. Roadster provides some default shutdown
    /// signals, but it may be desirable to provide a custom signal in order to, e.g., shutdown the
    /// server when a particular API is called.
    async fn graceful_shutdown_signal(self: Arc<Self>, _state: &S) {
        let _output: () = future::pending().await;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::PrepareOptions;
    use insta::assert_debug_snapshot;

    #[test]
    fn prepare_options_test() {
        let options = PrepareOptions::test();
        assert_debug_snapshot!(options);
    }
}
