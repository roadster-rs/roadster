pub mod context;
pub mod metadata;

#[cfg(feature = "cli")]
use crate::api::cli::parse_cli;
#[cfg(all(test, feature = "cli"))]
use crate::api::cli::MockTestCli;
#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::metadata::AppMetadata;
use crate::config::app_config::AppConfig;
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
use crate::error::RoadsterResult;
use crate::health_check::registry::HealthCheckRegistry;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
use axum::extract::FromRef;
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
use tracing::{instrument, warn};

pub async fn run<A, S>(
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] app: A,
) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Default + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = parse_cli::<A, S, _, _>(env::args_os())?;

    #[cfg(feature = "cli")]
    let environment = roadster_cli.environment.clone();
    #[cfg(not(feature = "cli"))]
    let environment: Option<Environment> = None;

    let config = AppConfig::new(environment)?;

    A::init_tracing(&config)?;

    #[cfg(not(feature = "cli"))]
    config.validate(true)?;
    #[cfg(feature = "cli")]
    config.validate(!roadster_cli.skip_validate_config)?;

    #[cfg(not(test))]
    let metadata = A::metadata(&config)?;

    // The `config.clone()` here is technically not necessary. However, without it, RustRover
    // is giving a "value used after move" error when creating an actual `AppContext` below.
    #[cfg(test)]
    let context = AppContext::test(Some(config.clone()), None, None)?;
    #[cfg(not(test))]
    let context = AppContext::new::<A, S>(config, metadata).await?;

    let state = A::provide_state(context.clone()).await?;

    let mut health_checks = HealthCheckRegistry::new(&context);
    A::health_checks(&mut health_checks, &state).await?;
    context.set_health_checks(health_checks)?;

    #[cfg(feature = "cli")]
    if crate::api::cli::handle_cli(&app, &roadster_cli, &app_cli, &state).await? {
        return Ok(());
    }

    let mut service_registry = ServiceRegistry::new(&state);
    A::services(&mut service_registry, &state).await?;

    if service_registry.services.is_empty() {
        warn!("No enabled services were registered, exiting.");
        return Ok(());
    }

    #[cfg(feature = "cli")]
    if crate::service::runner::handle_cli(&roadster_cli, &app_cli, &service_registry, &state)
        .await?
    {
        return Ok(());
    }

    #[cfg(feature = "db-sql")]
    if context.config().database.auto_migrate {
        A::M::up(context.db(), None).await?;
    }

    crate::service::runner::health_checks(&context).await?;

    crate::service::runner::before_run(&service_registry, &state).await?;

    crate::service::runner::run(service_registry, &state).await?;

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

    fn init_tracing(config: &AppConfig) -> RoadsterResult<()> {
        init_tracing(config, &Self::metadata(config)?)?;

        Ok(())
    }

    fn metadata(_config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(Default::default())
    }

    #[cfg(feature = "db-sql")]
    fn db_connection_options(config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        Ok(ConnectOptions::from(&config.database))
    }

    /// Provide the app state that will be used throughout the app. The state can simply be the
    /// provided [AppContext], or a custom type that implements [FromRef] to allow Roadster to
    /// extract its [AppContext] when needed.
    ///
    /// See the following for more details regarding [FromRef]: <https://docs.rs/axum/0.7.5/axum/extract/trait.FromRef.html>
    async fn provide_state(context: AppContext) -> RoadsterResult<S>;

    /// Provide the [crate::health_check::HealthCheck]s to use throughout the app.
    async fn health_checks(_registry: &mut HealthCheckRegistry, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }

    /// Provide the [crate::service::AppService]s to run in the app.
    async fn services(_registry: &mut ServiceRegistry<Self, S>, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }

    /// Override to provide a custom shutdown signal. Roadster provides some default shutdown
    /// signals, but it may be desirable to provide a custom signal in order to, e.g., shutdown the
    /// server when a particular API is called.
    async fn graceful_shutdown_signal(_state: &S) {
        let _output: () = future::pending().await;
    }

    /// Override to provide custom graceful shutdown logic to clean up any resources created by
    /// the app. Roadster will take care of cleaning up the resources it created.
    #[instrument(skip_all)]
    async fn graceful_shutdown(_state: &S) -> RoadsterResult<()> {
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
