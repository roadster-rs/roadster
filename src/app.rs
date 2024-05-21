#[mockall_double::double]
use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::parse_cli;
#[cfg(all(test, feature = "cli"))]
use crate::cli::MockCli;
#[cfg(feature = "cli")]
use crate::cli::RunCommand;
use crate::config::app_config::AppConfig;
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
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

// todo: this method is getting unweildy, we should break it up
pub async fn run<A>(
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] app: A,
) -> anyhow::Result<()>
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
    let context = AppContext::<()>::new::<A>(config).await?;
    #[cfg(test)]
    let context = AppContext::<()>::default();

    let state = A::with_state(&context).await?;
    let context = context.with_custom(state);

    #[cfg(feature = "cli")]
    crate::cli::handle_cli(&app, &roadster_cli, &app_cli, &context).await?;

    let mut service_registry = ServiceRegistry::new(&context);
    A::services(&mut service_registry, &context).await?;

    #[cfg(feature = "cli")]
    crate::service::runner::handle_cli(&roadster_cli, &app_cli, &service_registry, &context)
        .await?;

    if service_registry.services.is_empty() {
        warn!("No enabled services were registered, exiting.");
        return Ok(());
    }

    #[cfg(feature = "db-sql")]
    if context.config().database.auto_migrate {
        A::M::up(context.db(), None).await?;
    }

    crate::service::runner::run(service_registry, &context).await?;

    Ok(())
}

#[async_trait]
pub trait App: Send + Sync {
    // Todo: Are clone, etc necessary if we store it inside an Arc?
    type State: Clone + Send + Sync + 'static;
    #[cfg(feature = "cli")]
    type Cli: clap::Args + RunCommand<Self>;
    #[cfg(feature = "db-sql")]
    type M: MigratorTrait;

    fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
    }

    #[cfg(feature = "db-sql")]
    fn db_connection_options(config: &AppConfig) -> anyhow::Result<ConnectOptions> {
        let mut options = ConnectOptions::new(config.database.uri.to_string());
        options
            .connect_timeout(config.database.connect_timeout)
            .acquire_timeout(config.database.acquire_timeout)
            .min_connections(config.database.min_connections)
            .max_connections(config.database.max_connections)
            .sqlx_logging(false);
        if let Some(idle_timeout) = config.database.idle_timeout {
            options.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = config.database.max_lifetime {
            options.max_lifetime(max_lifetime);
        }
        Ok(options)
    }

    /// Convert the [AppContext] to the custom [Self::State] that will be used throughout the app.
    /// The conversion can simply happen in a [`From<AppContext>`] implementation, but this
    /// method is provided in case there's any additional work that needs to be done that the
    /// consumer can't put in a [`From<AppContext>`] implementation. For example, any
    /// configuration that needs to happen in an async method.
    async fn with_state(context: &AppContext<()>) -> anyhow::Result<Self::State>;

    /// Provide the services to run in the app.
    async fn services(
        _registry: &mut ServiceRegistry<Self>,
        _context: &AppContext<Self::State>,
    ) -> anyhow::Result<()> {
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
    async fn graceful_shutdown(_context: &AppContext<Self::State>) -> anyhow::Result<()> {
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

#[cfg(test)]
mockall::mock! {
    pub TestApp {}
    #[async_trait]
    impl App for TestApp {
        type State = ();
        #[cfg(feature = "cli")]
        type Cli = MockCli;
        #[cfg(feature = "db-sql")]
        type M = MockMigrator;

        async fn with_state(context: &AppContext<()>) -> anyhow::Result<<MockTestApp as App>::State>;
    }
}
