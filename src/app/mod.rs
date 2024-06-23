pub mod context;
pub mod metadata;

#[cfg(feature = "cli")]
use crate::api::cli::parse_cli;
#[cfg(all(test, feature = "cli"))]
use crate::api::cli::MockCli;
#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::metadata::AppMetadata;
use crate::config::app_config::AppConfig;
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
use crate::error::RoadsterResult;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
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

pub async fn run<A>(
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] app: A,
) -> RoadsterResult<()>
where
    A: App + Default + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = parse_cli::<A, _, _>(env::args_os())?;

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
    let context = AppContext::<()>::test(Some(config.clone()), None, None)?;
    #[cfg(not(test))]
    let context = AppContext::<()>::new::<A>(config, metadata).await?;

    let state = A::with_state(&context).await?;
    let context = context.with_custom(state);

    #[cfg(feature = "cli")]
    if crate::api::cli::handle_cli(&app, &roadster_cli, &app_cli, &context).await? {
        return Ok(());
    }

    let mut service_registry = ServiceRegistry::new(&context);
    A::services(&mut service_registry, &context).await?;

    if service_registry.services.is_empty() {
        warn!("No enabled services were registered, exiting.");
        return Ok(());
    }

    #[cfg(feature = "cli")]
    if crate::service::runner::handle_cli(&roadster_cli, &app_cli, &service_registry, &context)
        .await?
    {
        return Ok(());
    }

    #[cfg(feature = "db-sql")]
    if context.config().database.auto_migrate {
        A::M::up(context.db(), None).await?;
    }

    crate::service::runner::health_checks(&service_registry, &context).await?;

    crate::service::runner::before_run(&service_registry, &context).await?;

    crate::service::runner::run(service_registry, &context).await?;

    Ok(())
}

#[cfg_attr(test, mockall::automock(type State = (); type Cli = MockCli; type M = MockMigrator;))]
#[async_trait]
pub trait App: Send + Sync {
    // Todo: Are clone, etc necessary if we store it inside an Arc?
    type State: Clone + Send + Sync + 'static;
    #[cfg(feature = "cli")]
    type Cli: clap::Args + RunCommand<Self>;
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

    /// Convert the [AppContext] to the custom [Self::State] that will be used throughout the app.
    /// The conversion can simply happen in a [`From<AppContext>`] implementation, but this
    /// method is provided in case there's any additional work that needs to be done that the
    /// consumer can't put in a [`From<AppContext>`] implementation. For example, any
    /// configuration that needs to happen in an async method.
    async fn with_state(context: &AppContext<()>) -> RoadsterResult<Self::State>;

    /// Provide the services to run in the app.
    async fn services(
        _registry: &mut ServiceRegistry<Self>,
        _context: &AppContext<Self::State>,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// Override to provide a custom shutdown signal. Roadster provides some default shutdown
    /// signals, but it may be desirable to provide a custom signal in order to, e.g., shutdown the
    /// server when a particular API is called.
    async fn graceful_shutdown_signal(_context: &AppContext<Self::State>) {
        let _output: () = future::pending().await;
    }

    /// Override to provide custom graceful shutdown logic to clean up any resources created by
    /// the app. Roadster will take care of cleaning up the resources it created.
    #[instrument(skip_all)]
    async fn graceful_shutdown(_context: &AppContext<Self::State>) -> RoadsterResult<()> {
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
