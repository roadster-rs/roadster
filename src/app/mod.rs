pub mod context;
pub mod metadata;
mod roadster_app;

/// A default implementation of [`App`] that is customizable via a builder-style API.
///
/// See <https://github.com/roadster-rs/roadster/tree/main/examples/app-builder/src/main.rs> for
/// an example of how to use the [`RoadsterApp`].
///
/// The `Cli` and `M` type parameters are only required when the `cli` and `db-sql` features are
/// enabled, respectively.
pub use roadster_app::RoadsterApp;

/// Builder-style API to build/customize a [`RoadsterApp`].
///
/// See <https://github.com/roadster-rs/roadster/tree/main/examples/app-builder/src/main.rs> for
/// an example of how to use the [`RoadsterAppBuilder`].
///
/// The `Cli` and `M` type parameters are only required when the `cli` and `db-sql` features are
/// enabled, respectively.
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
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
use crate::config::{AppConfig, AppConfigOptions};
use crate::error::RoadsterResult;
use crate::health_check::registry::HealthCheckRegistry;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use context::AppContext;
#[cfg(feature = "db-sql")]
use sea_orm::ConnectOptions;
#[cfg(all(test, feature = "db-sql"))]
use sea_orm_migration::MigrationTrait;
#[cfg(feature = "db-sql")]
use sea_orm_migration::MigratorTrait;
#[cfg(feature = "cli")]
use std::env;
use std::future;
use std::sync::Arc;
use tracing::{error, info, instrument, warn};

pub async fn run<A, S>(app: A) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let cli_and_state = build_cli_and_state(app).await?;

    #[cfg(feature = "cli")]
    {
        let CliAndState {
            app,
            #[cfg(feature = "cli")]
            roadster_cli,
            #[cfg(feature = "cli")]
            app_cli,
            state,
        } = &cli_and_state;

        if crate::api::cli::handle_cli(app, roadster_cli, app_cli, state).await? {
            return Ok(());
        }
    }

    let prepared = prepare_from_cli_and_state(cli_and_state).await?;

    if run_prepared_service_cli(&prepared).await? {
        return Ok(());
    }

    run_prepared_without_cli(prepared).await
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

async fn build_cli_and_state<A, S>(app: A) -> RoadsterResult<CliAndState<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = parse_cli::<A, S, _, _>(env::args_os())?;

    #[cfg(feature = "cli")]
    let environment = roadster_cli.environment.clone();
    #[cfg(not(feature = "cli"))]
    let environment: Option<Environment> = None;

    #[cfg(feature = "cli")]
    let config_dir = roadster_cli.config_dir.as_ref().cloned();
    #[cfg(not(feature = "cli"))]
    let config_dir: Option<std::path::PathBuf> = None;

    let config = AppConfig::new_with_options(
        AppConfigOptions::builder()
            .environment_opt(environment)
            .config_dir_opt(config_dir)
            .build(),
    )?;

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
    pub app: A,
    #[cfg(feature = "cli")]
    pub roadster_cli: RoadsterCli,
    #[cfg(feature = "cli")]
    pub app_cli: A::Cli,
    pub state: S,
    pub health_check_registry: HealthCheckRegistry,
    pub service_registry: ServiceRegistry<A, S>,
    pub lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

#[non_exhaustive]
struct PreparedAppWithoutCli<A, S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    health_check_registry: HealthCheckRegistry,
    service_registry: ServiceRegistry<A, S>,
    lifecycle_handler_registry: LifecycleHandlerRegistry<A, S>,
}

/// Prepare the app. Sets up everything needed to start the app, but does not execute anything.
/// Specifically, the following are skipped:
///
/// 1. Handling CLI commands
/// 2. Health checks
/// 3. Lifecycle Handlers
/// 4. Starting any services
pub async fn prepare<A, S>(app: A) -> RoadsterResult<PreparedApp<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    prepare_from_cli_and_state(build_cli_and_state(app).await?).await
}

/// Initialize the app state. Does everything to initialize the app short of starting the app.
/// Similar to [`prepare`], except performs some steps that are skipped in [`prepare`]:
/// 1. Health checks
/// 2. Lifecycle Handlers
///
/// The following are still skipped:
/// 1. Handling CLI commands
/// 2. Starting any services
///
/// Additionally, the health checks are not attached to the [`AppContext`] to avoid a reference
/// cycle that prevents the [`AppContext`] from being dropped between tests.
///
/// This is intended to only be used to get access to the app's fully set up state in tests.
#[cfg(feature = "testing")]
pub async fn init_state<A, S>(app: &A, config: AppConfig) -> RoadsterResult<S>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let state = build_state(app, config).await?;

    let PreparedAppWithoutCli {
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
    } = prepare_without_cli(app, &state).await?;
    before_app(
        &state,
        Some(health_check_registry),
        &service_registry,
        &lifecycle_handler_registry,
    )
    .await?;

    Ok(state)
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
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
    } = prepare_without_cli(&app, &state).await?;

    Ok(PreparedApp {
        app,
        #[cfg(feature = "cli")]
        roadster_cli,
        #[cfg(feature = "cli")]
        app_cli,
        state,
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
    })
}

async fn prepare_without_cli<A, S>(
    app: &A,
    state: &S,
) -> RoadsterResult<PreparedAppWithoutCli<A, S>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let context = AppContext::from_ref(state);

    let mut lifecycle_handler_registry = LifecycleHandlerRegistry::new(state);
    app.lifecycle_handlers(&mut lifecycle_handler_registry, state)
        .await?;

    let mut health_check_registry = HealthCheckRegistry::new(&context);
    app.health_checks(&mut health_check_registry, state).await?;
    // Note that we used to set the health check registry on the `AppContext` here. However, we
    // don't do that anymore because it causes a reference cycle between the `AppContext` and the
    // `HealthChecks` (at least the ones that contain an `AppContext`). This shouldn't normally
    // be a problem, but it causes TestContainers containers to not be cleaned up in tests.
    // We may want to re-think some designs to avoid this reference cycle.

    let mut service_registry = ServiceRegistry::new(state);
    app.services(&mut service_registry, state).await?;

    Ok(PreparedAppWithoutCli {
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
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
        let PreparedApp {
            app,
            roadster_cli,
            app_cli,
            state,
            ..
        } = &prepared_app;
        if crate::api::cli::handle_cli(app, roadster_cli, app_cli, state).await? {
            return Ok(());
        }
    }

    if run_prepared_service_cli(&prepared_app).await? {
        return Ok(());
    }

    run_prepared_without_cli(prepared_app).await
}

/// Run a [PreparedApp] that was previously crated by [prepare]
async fn run_prepared_service_cli<A, S>(prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let service_registry = &prepared_app.service_registry;

    if service_registry.services.is_empty() {
        warn!("No enabled services were registered.");
        return Ok(false);
    }

    #[cfg(feature = "cli")]
    {
        let state = &prepared_app.state;
        let lifecycle_handlers = prepared_app.lifecycle_handler_registry.handlers(state);
        info!("Running AppLifecycleHandler::before_service_cli");
        for handler in lifecycle_handlers.iter() {
            info!(name=%handler.name(), "Running AppLifecycleHandler::before_service_cli");
            handler.before_service_cli(state).await?;
        }

        let PreparedApp {
            roadster_cli,
            app_cli,
            ..
        } = &prepared_app;
        if crate::service::runner::handle_cli(roadster_cli, app_cli, service_registry, state)
            .await?
        {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Run the app's initialization logic (lifecycle handlers, health checks, etc).
async fn before_app<A, S>(
    state: &S,
    health_check_registry: Option<HealthCheckRegistry>,
    service_registry: &ServiceRegistry<A, S>,
    lifecycle_handler_registry: &LifecycleHandlerRegistry<A, S>,
) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    if service_registry.services.is_empty() {
        warn!("No enabled services were registered.");
    }

    let lifecycle_handlers = lifecycle_handler_registry.handlers(state);

    info!("Running AppLifecycleHandler::before_health_checks");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_health_checks");
        handler.before_health_checks(state).await?;
    }
    let checks = if let Some(health_check_registry) = health_check_registry {
        health_check_registry.checks()
    } else {
        let context = AppContext::from_ref(state);
        context.health_checks()
    };
    crate::service::runner::health_checks(checks).await?;

    info!("Running AppLifecycleHandler::before_services");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_services");
        handler.before_services(state).await?
    }
    crate::service::runner::before_run(service_registry, state).await?;

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
    let PreparedApp {
        app,
        state,
        health_check_registry,
        service_registry,
        lifecycle_handler_registry,
        ..
    } = prepared_app;

    let context = AppContext::from_ref(&state);
    context.set_health_checks(health_check_registry)?;

    before_app(&state, None, &service_registry, &lifecycle_handler_registry).await?;

    let result = crate::service::runner::run(app, service_registry, &state).await;
    if let Err(err) = result {
        error!("An error occurred in the app: {err}");
    }

    info!("Shutting down");

    let lifecycle_handlers = lifecycle_handler_registry.handlers(&state);

    info!("Running AppLifecycleHandler::before_shutdown");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_shutdown");
        let result = handler.on_shutdown(&state).await;
        if let Err(err) = result {
            error!(name=%handler.name(), "An error occurred when running AppLifecycleHandler::before_shutdown: {err}");
        }
    }

    info!("Shutdown complete");

    Ok(())
}

#[cfg_attr(test, mockall::automock(type Cli = MockTestCli<S>; type M = MockMigrator;))]
#[async_trait]
pub trait App<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    type Cli: clap::Args + RunCommand<Self, S> + Send + Sync;
    #[cfg(feature = "db-sql")]
    type M: MigratorTrait;

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        init_tracing(config, &self.metadata(config)?)?;

        Ok(())
    }

    fn metadata(&self, _config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(Default::default())
    }

    #[cfg(feature = "db-sql")]
    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        Ok(ConnectOptions::from(&config.database))
    }

    /// Provide the app state that will be used throughout the app. The state can simply be the
    /// provided [AppContext], or a custom type that implements [FromRef] to allow Roadster to
    /// extract its [AppContext] when needed.
    ///
    /// See the following for more details regarding [FromRef]: <https://docs.rs/axum/0.7.5/axum/extract/trait.FromRef.html>
    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S>;

    async fn lifecycle_handlers(
        &self,
        _registry: &mut LifecycleHandlerRegistry<Self, S>,
        _state: &S,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// Provide the [crate::health_check::HealthCheck]s to use throughout the app.
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

    /// Override to provide custom graceful shutdown logic to clean up any resources created by
    /// the app. Roadster will take care of cleaning up the resources it created.
    ///
    /// Alternatively, provide a [`crate::lifecycle::AppLifecycleHandler::on_shutdown`]
    /// implementation and provide the handler to the [`LifecycleHandlerRegistry`] in
    /// [`Self::lifecycle_handlers`].
    ///
    /// This method is intentionally not provided in the builder-style API of [`RoadsterApp`]; it's
    /// expected that consumers would provide their shutdown logic in a
    /// [`crate::lifecycle::AppLifecycleHandler::on_shutdown`] implementation instead.
    #[instrument(skip_all)]
    async fn graceful_shutdown(self: Arc<Self>, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }
}

#[cfg(all(test, feature = "db-sql"))]
mockall::mock! {
    pub Migrator {}
    #[async_trait]
    impl MigratorTrait for Migrator {
        fn migrations() -> Vec<Box<dyn MigrationTrait>>;
    }
}
