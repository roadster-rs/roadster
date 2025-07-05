pub mod context;
pub mod metadata;
mod prepare;
mod roadster_app;
mod run;
#[cfg(feature = "testing")]
mod test;

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

pub use prepare::{PrepareOptions, PreparedApp, PreparedAppCli, PreparedAppWithoutCli, prepare};
pub use run::{run, run_prepared};
#[cfg(feature = "testing")]
pub use test::{TestAppState, run_test, run_test_with_result, test_state};

#[cfg(all(test, feature = "cli"))]
use crate::api::cli::MockTestCli;
#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::context::extension::ExtensionRegistry;
use crate::app::metadata::AppMetadata;
use crate::config::AppConfig;
use crate::config::environment::Environment;
#[cfg(feature = "db-sql")]
use crate::db::migration::Migrator;
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use context::AppContext;
#[cfg(feature = "db-sea-orm")]
use sea_orm::ConnectOptions;
use std::future;
use std::sync::Arc;

#[cfg_attr(all(test, feature = "cli"), mockall::automock(type Cli = MockTestCli<S>;))]
#[cfg_attr(
    all(test, not(feature = "cli")),
    mockall::automock(type Cli = crate::util::empty::Empty;)
)]
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

    async fn provide_context_extensions(
        &self,
        _config: &AppConfig,
        _extension_registry: &mut ExtensionRegistry,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        Ok(ConnectOptions::from(&config.database))
    }

    #[cfg(feature = "db-diesel-pool")]
    fn diesel_connection_customizer<C>(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<Option<Box<dyn r2d2::CustomizeConnection<C, diesel::r2d2::Error>>>>
    where
        C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
    {
        Ok(None)
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    fn diesel_pg_connection_customizer(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselPgConn, diesel::r2d2::Error>>,
    > {
        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    fn diesel_mysql_connection_customizer(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselMysqlConn, diesel::r2d2::Error>>,
    > {
        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    fn diesel_sqlite_connection_customizer(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselSqliteConn, diesel::r2d2::Error>>,
    > {
        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    fn diesel_pg_async_connection_customizer(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<
        Box<
            dyn bb8::CustomizeConnection<
                    crate::db::DieselPgConnAsync,
                    diesel_async::pooled_connection::PoolError,
                >,
        >,
    > {
        Ok(Box::new(crate::util::empty::Empty))
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    fn diesel_mysql_async_connection_customizer(
        &self,
        _config: &AppConfig,
    ) -> RoadsterResult<
        Box<
            dyn bb8::CustomizeConnection<
                    crate::db::DieselMysqlConnAsync,
                    diesel_async::pooled_connection::PoolError,
                >,
        >,
    > {
        Ok(Box::new(crate::util::empty::Empty))
    }

    /// Allows customizing the pool options used for the `sqlx` Postgres connection used for `pgmq`.
    #[cfg(feature = "worker-pg")]
    fn worker_pg_sqlx_pool_options(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<sqlx::pool::PoolOptions<sqlx::Postgres>> {
        if let Some(pool_config) = config
            .service
            .worker
            .pg
            .custom
            .custom
            .db_config
            .as_ref()
            .and_then(|config| config.pool_config.as_ref())
        {
            Ok(pool_config.into())
        } else {
            Ok((&config.database).into())
        }
    }

    /// Provide the app state that will be used throughout the app. The state can simply be the
    /// provided [`AppContext`], or a custom type that implements [`FromRef`] to allow Roadster to
    /// extract its [`AppContext`] when needed.
    ///
    /// See the following for more details regarding [`FromRef`]: <https://docs.rs/axum/0.7.5/axum/extract/trait.FromRef.html>
    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S>;

    /// Note: SeaORM and Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so multiple SeaORM or Diesel migrators should not be provided
    /// via this method.
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
