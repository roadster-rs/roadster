#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::context::AppContext;
use crate::app::metadata::AppMetadata;
use crate::app::{App, run};
use crate::config::AppConfig;
use crate::config::environment::Environment;
#[cfg(feature = "db-sql")]
use crate::db::migration::Migrator;
use crate::error::RoadsterResult;
use crate::health::check::HealthCheck;
use crate::health::check::registry::HealthCheckRegistry;
use crate::lifecycle::AppLifecycleHandler;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::service::AppService;
use crate::service::registry::ServiceRegistry;
use crate::util::empty::Empty;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use config::AsyncSource;
#[cfg(feature = "db-sea-orm")]
use sea_orm::ConnectOptions;
use std::future;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

type StateBuilder<S> = dyn Send + Sync + Fn(AppContext) -> RoadsterResult<S>;
type TracingInitializer = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>;
type AsyncConfigSourceProvider =
    dyn Send + Sync + Fn(&Environment) -> RoadsterResult<Box<dyn AsyncSource + Send + Sync>>;
type MetadataProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>;
#[cfg(feature = "db-sea-orm")]
type DbConnOptionsProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>;
#[cfg(feature = "worker-pg")]
type SqlxPgPoolOptionsProvider =
    dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<sqlx::pool::PoolOptions<sqlx::Postgres>>;
#[cfg(any(
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool"
))]
type DieselConnectionCustomizer<C> = Box<dyn r2d2::CustomizeConnection<C, diesel::r2d2::Error>>;
#[cfg(any(
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool"
))]
type DieselConnectionCustomizerProvider<C> =
    dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<DieselConnectionCustomizer<C>>;
#[cfg(any(
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async"
))]
type DieselAsyncConnectionCustomizer<C> =
    Box<dyn bb8_8::CustomizeConnection<C, diesel_async::pooled_connection::PoolError>>;
#[cfg(any(
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async"
))]
type DieselAsyncConnectionCustomizerProvider<C> =
    dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<DieselAsyncConnectionCustomizer<C>>;
#[cfg(feature = "db-sql")]
type MigratorProvider<S> = dyn Send + Sync + Fn(&S) -> RoadsterResult<Box<dyn Migrator<S>>>;
type LifecycleHandlers<A, S> = Vec<Box<dyn AppLifecycleHandler<A, S>>>;
type LifecycleHandlerProviders<A, S> =
    Vec<Box<dyn Send + Sync + Fn(&mut LifecycleHandlerRegistry<A, S>, &S) -> RoadsterResult<()>>>;
type HealthCheckProviders<S> =
    Vec<Box<dyn Send + Sync + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>>>;
type Services<A, S> = Vec<Box<dyn AppService<A, S>>>;
type ServiceProviders<A, S> = Vec<
    Box<
        dyn Send
            + Sync
            + for<'a> Fn(
                &'a mut ServiceRegistry<A, S>,
                &'a S,
            ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
    >,
>;
type GracefulShutdownSignalProvider<S> =
    Option<Box<dyn Send + Sync + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>>>;

/// Inner state shared between both the [`RoadsterApp`] and [`RoadsterAppBuilder`].
struct Inner<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
> where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    state_provider: Option<Box<StateBuilder<S>>>,
    tracing_initializer: Option<Box<TracingInitializer>>,
    async_config_source_providers: Vec<Box<AsyncConfigSourceProvider>>,
    metadata: Option<AppMetadata>,
    metadata_provider: Option<Box<MetadataProvider>>,
    #[cfg(feature = "db-sea-orm")]
    sea_orm_conn_options: Option<ConnectOptions>,
    #[cfg(feature = "db-sea-orm")]
    sea_orm_conn_options_provider: Option<Box<DbConnOptionsProvider>>,
    #[cfg(feature = "db-diesel-postgres-pool")]
    diesel_pg_connection_customizer_provider:
        Option<Box<DieselConnectionCustomizerProvider<crate::db::DieselPgConn>>>,
    #[cfg(feature = "db-diesel-mysql-pool")]
    diesel_mysql_connection_customizer_provider:
        Option<Box<DieselConnectionCustomizerProvider<crate::db::DieselMysqlConn>>>,
    #[cfg(feature = "db-diesel-sqlite-pool")]
    diesel_sqlite_connection_customizer_provider:
        Option<Box<DieselConnectionCustomizerProvider<crate::db::DieselSqliteConn>>>,
    #[cfg(feature = "db-diesel-postgres-pool-async")]
    diesel_pg_async_connection_customizer_provider:
        Option<Box<DieselAsyncConnectionCustomizerProvider<crate::db::DieselPgConnAsync>>>,
    #[cfg(feature = "db-diesel-mysql-pool-async")]
    diesel_mysql_async_connection_customizer_provider:
        Option<Box<DieselAsyncConnectionCustomizerProvider<crate::db::DieselMysqlConnAsync>>>,
    #[cfg(feature = "worker-pg")]
    worker_pg_sqlx_pool_options: Option<sqlx::pool::PoolOptions<sqlx::Postgres>>,
    #[cfg(feature = "worker-pg")]
    worker_pg_sqlx_pool_options_provider: Option<Box<SqlxPgPoolOptionsProvider>>,
    #[cfg(feature = "db-sql")]
    migrator_providers: Vec<Box<MigratorProvider<S>>>,
    health_checks: Vec<Arc<dyn HealthCheck>>,
    health_check_providers: HealthCheckProviders<S>,
    graceful_shutdown_signal_provider: GracefulShutdownSignalProvider<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S, Cli>, S>,
    service_providers: ServiceProviders<RoadsterApp<S, Cli>, S>,
}

impl<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
    #[cfg(not(feature = "cli"))] Cli: 'static,
> Inner<S, Cli>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    fn new() -> Self {
        Self {
            state_provider: Default::default(),
            tracing_initializer: Default::default(),
            async_config_source_providers: Default::default(),
            metadata: Default::default(),
            metadata_provider: Default::default(),
            #[cfg(feature = "db-sea-orm")]
            sea_orm_conn_options: Default::default(),
            #[cfg(feature = "db-sea-orm")]
            sea_orm_conn_options_provider: Default::default(),
            #[cfg(feature = "db-diesel-postgres-pool")]
            diesel_pg_connection_customizer_provider: Default::default(),
            #[cfg(feature = "db-diesel-mysql-pool")]
            diesel_mysql_connection_customizer_provider: Default::default(),
            #[cfg(feature = "db-diesel-sqlite-pool")]
            diesel_sqlite_connection_customizer_provider: Default::default(),
            #[cfg(feature = "db-diesel-postgres-pool-async")]
            diesel_pg_async_connection_customizer_provider: Default::default(),
            #[cfg(feature = "db-diesel-mysql-pool-async")]
            diesel_mysql_async_connection_customizer_provider: Default::default(),
            #[cfg(feature = "db-sea-orm")]
            worker_pg_sqlx_pool_options: Default::default(),
            #[cfg(feature = "db-sea-orm")]
            worker_pg_sqlx_pool_options_provider: Default::default(),
            #[cfg(feature = "db-sql")]
            migrator_providers: Default::default(),
            health_checks: Default::default(),
            health_check_providers: Default::default(),
            graceful_shutdown_signal_provider: Default::default(),
            lifecycle_handler_providers: Default::default(),
            service_providers: Default::default(),
        }
    }

    fn tracing_initializer(
        &mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) {
        self.tracing_initializer = Some(Box::new(tracing_initializer));
    }

    fn add_async_config_source_provider(
        &mut self,
        async_config_source_provider: impl 'static
        + Send
        + Sync
        + Fn(
            &Environment,
        )
            -> RoadsterResult<Box<dyn AsyncSource + Send + Sync>>,
    ) {
        self.async_config_source_providers
            .push(Box::new(async_config_source_provider));
    }

    fn set_metadata(&mut self, metadata: AppMetadata) {
        self.metadata = Some(metadata);
    }

    fn metadata_provider(
        &mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) {
        self.metadata_provider = Some(Box::new(metadata_provider));
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_conn_options(&mut self, sea_orm_conn_options: ConnectOptions) {
        self.sea_orm_conn_options = Some(sea_orm_conn_options);
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_conn_options_provider(
        &mut self,
        sea_orm_conn_options_provider: impl 'static
        + Send
        + Sync
        + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) {
        self.sea_orm_conn_options_provider = Some(Box::new(sea_orm_conn_options_provider));
    }

    fn state_provider(
        &mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) {
        self.state_provider = Some(Box::new(builder));
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    fn diesel_pg_connection_customizer_provider(
        &mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselPgConn, diesel::r2d2::Error>>,
        >,
    ) {
        self.diesel_pg_connection_customizer_provider = Some(Box::new(connection_customizer));
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    fn diesel_mysql_connection_customizer_provider(
        &mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselMysqlConn, diesel::r2d2::Error>>,
        >,
    ) {
        self.diesel_mysql_connection_customizer_provider = Some(Box::new(connection_customizer));
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    fn diesel_sqlite_connection_customizer_provider(
        &mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselSqliteConn, diesel::r2d2::Error>>,
        >,
    ) {
        self.diesel_sqlite_connection_customizer_provider = Some(Box::new(connection_customizer));
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    fn diesel_pg_async_connection_customizer_provider(
        &mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<
                dyn bb8_8::CustomizeConnection<
                        crate::db::DieselPgConnAsync,
                        diesel_async::pooled_connection::PoolError,
                    >,
            >,
        >,
    ) {
        self.diesel_pg_async_connection_customizer_provider = Some(Box::new(connection_customizer));
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    fn diesel_mysql_async_connection_customizer_provider(
        &mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<
                dyn bb8_8::CustomizeConnection<
                        crate::db::DieselMysqlConnAsync,
                        diesel_async::pooled_connection::PoolError,
                    >,
            >,
        >,
    ) {
        self.diesel_mysql_async_connection_customizer_provider =
            Some(Box::new(connection_customizer));
    }

    #[cfg(feature = "worker-pg")]
    fn worker_pg_sqlx_pool_options(
        &mut self,
        worker_pg_sqlx_pool_options: sqlx::pool::PoolOptions<sqlx::Postgres>,
    ) {
        self.worker_pg_sqlx_pool_options = Some(worker_pg_sqlx_pool_options);
    }

    #[cfg(feature = "worker-pg")]
    fn worker_pg_sqlx_pool_options_provider(
        &mut self,
        worker_pg_sqlx_pool_options_provider: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            sqlx::pool::PoolOptions<sqlx::Postgres>,
        >,
    ) {
        self.worker_pg_sqlx_pool_options_provider =
            Some(Box::new(worker_pg_sqlx_pool_options_provider));
    }

    #[cfg(feature = "db-sql")]
    fn add_migrator_provider(
        &mut self,
        migrator_provider: impl 'static + Send + Sync + Fn(&S) -> RoadsterResult<Box<dyn Migrator<S>>>,
    ) {
        self.migrator_providers.push(Box::new(migrator_provider))
    }

    fn add_health_check(&mut self, health_check: impl 'static + HealthCheck) {
        self.health_checks.push(Arc::new(health_check));
    }

    fn add_health_check_provider(
        &mut self,
        health_check_provider: impl 'static
        + Send
        + Sync
        + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) {
        self.health_check_providers
            .push(Box::new(health_check_provider));
    }

    fn provide_graceful_shutdown_signal(
        &mut self,
        graceful_shutdown_signal_provider: impl 'static
        + Send
        + Sync
        + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) {
        self.graceful_shutdown_signal_provider = Some(Box::new(graceful_shutdown_signal_provider));
    }

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        if let Some(tracing_initializer) = self.tracing_initializer.as_ref() {
            tracing_initializer(config)
        } else {
            crate::tracing::init_tracing(config, &self.get_metadata(config)?)
        }
    }

    fn get_metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        if let Some(metadata) = self.metadata.as_ref() {
            Ok(metadata.clone())
        } else if let Some(metadata_provider) = self.metadata_provider.as_ref() {
            metadata_provider(config)
        } else {
            Ok(Default::default())
        }
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        if let Some(sea_orm_conn_options) = self.sea_orm_conn_options.as_ref() {
            Ok(sea_orm_conn_options.clone())
        } else if let Some(sea_orm_conn_options_provider) =
            self.sea_orm_conn_options_provider.as_ref()
        {
            sea_orm_conn_options_provider(config)
        } else {
            Ok(ConnectOptions::from(&config.database))
        }
    }

    #[cfg(feature = "worker-pg")]
    fn build_worker_pg_sqlx_pool_options(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<sqlx::pool::PoolOptions<sqlx::Postgres>> {
        if let Some(worker_pg_sqlx_pool_options) = self.worker_pg_sqlx_pool_options.as_ref() {
            Ok(worker_pg_sqlx_pool_options.clone())
        } else if let Some(worker_pg_sqlx_pool_options_provider) =
            self.worker_pg_sqlx_pool_options_provider.as_ref()
        {
            worker_pg_sqlx_pool_options_provider(config)
        } else if let Some(pool_config) = &config.service.worker_pg.custom.db_pool {
            Ok(pool_config.into())
        } else {
            Ok((&config.database).into())
        }
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        let state_provider = self.state_provider.as_ref().ok_or_else(|| {
            crate::error::other::OtherError::Message("State builder missing".to_string())
        })?;
        state_provider(context)
    }

    async fn health_checks(
        &self,
        registry: &mut HealthCheckRegistry,
        state: &S,
    ) -> RoadsterResult<()> {
        for health_check in self.health_checks.iter() {
            registry.register_arc(health_check.clone())?;
        }
        for provider in self.health_check_providers.iter() {
            provider(registry, state)?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(&self, state: &S) {
        if let Some(signal) = self.graceful_shutdown_signal_provider.as_ref() {
            signal(state).await;
        } else {
            let _output: () = future::pending().await;
        }
    }
}

pub struct RoadsterApp<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
> where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S, Cli>,
    async_config_sources: Mutex<Vec<Box<dyn AsyncSource + Send + Sync>>>,
    #[cfg(feature = "db-sea-orm")]
    sea_orm_migrator: Mutex<Option<Box<dyn Migrator<S>>>>,
    #[cfg(feature = "db-diesel")]
    diesel_migrator: Mutex<Option<Box<dyn Migrator<S>>>>,
    #[cfg(feature = "db-sql")]
    migrators: Mutex<Vec<Box<dyn Migrator<S>>>>,
    #[cfg(feature = "db-diesel-postgres-pool")]
    diesel_pg_connection_customizer:
        Mutex<Option<DieselConnectionCustomizer<crate::db::DieselPgConn>>>,
    #[cfg(feature = "db-diesel-mysql-pool")]
    diesel_mysql_connection_customizer:
        Mutex<Option<DieselConnectionCustomizer<crate::db::DieselMysqlConn>>>,
    #[cfg(feature = "db-diesel-sqlite-pool")]
    diesel_sqlite_connection_customizer:
        Mutex<Option<DieselConnectionCustomizer<crate::db::DieselSqliteConn>>>,
    #[cfg(feature = "db-diesel-postgres-pool-async")]
    diesel_pg_async_connection_customizer:
        Mutex<Option<DieselAsyncConnectionCustomizer<crate::db::DieselPgConnAsync>>>,
    #[cfg(feature = "db-diesel-mysql-pool-async")]
    diesel_mysql_async_connection_customizer:
        Mutex<Option<DieselAsyncConnectionCustomizer<crate::db::DieselMysqlConnAsync>>>,
    // Interior mutability pattern -- this allows us to keep the handler reference as a
    // Box, which helps with single ownership and ensuring we only register a handler once.
    lifecycle_handlers: Mutex<LifecycleHandlers<RoadsterApp<S, Cli>, S>>,
    // Interior mutability pattern -- this allows us to keep the service reference as a
    // Box, which helps with single ownership and ensuring we only register a service once.
    services: Mutex<Services<RoadsterApp<S, Cli>, S>>,
}

pub struct RoadsterAppBuilder<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
> where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S, Cli>,
    async_config_sources: Vec<Box<dyn AsyncSource + Send + Sync>>,
    #[cfg(feature = "db-sea-orm")]
    sea_orm_migrator: Option<Box<dyn Migrator<S>>>,
    #[cfg(feature = "db-diesel")]
    diesel_migrator: Option<Box<dyn Migrator<S>>>,
    #[cfg(feature = "db-sql")]
    migrators: Vec<Box<dyn Migrator<S>>>,
    #[cfg(feature = "db-diesel-postgres-pool")]
    diesel_pg_connection_customizer: Option<DieselConnectionCustomizer<crate::db::DieselPgConn>>,
    #[cfg(feature = "db-diesel-mysql-pool")]
    diesel_mysql_connection_customizer:
        Option<DieselConnectionCustomizer<crate::db::DieselMysqlConn>>,
    #[cfg(feature = "db-diesel-sqlite-pool")]
    diesel_sqlite_connection_customizer:
        Option<DieselConnectionCustomizer<crate::db::DieselSqliteConn>>,
    #[cfg(feature = "db-diesel-postgres-pool-async")]
    diesel_pg_async_connection_customizer:
        Option<DieselAsyncConnectionCustomizer<crate::db::DieselPgConnAsync>>,
    #[cfg(feature = "db-diesel-mysql-pool-async")]
    diesel_mysql_async_connection_customizer:
        Option<DieselAsyncConnectionCustomizer<crate::db::DieselMysqlConnAsync>>,
    lifecycle_handlers: LifecycleHandlers<RoadsterApp<S, Cli>, S>,
    services: Services<RoadsterApp<S, Cli>, S>,
}

impl<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
    #[cfg(not(feature = "cli"))] Cli: 'static,
> RoadsterApp<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    /// Create a new [`RoadsterAppBuilder`] to use to build the [`RoadsterApp`].
    pub fn builder() -> RoadsterAppBuilder<S, Cli> {
        RoadsterAppBuilder::new()
    }

    /// Utility method to run the [`RoadsterApp`].
    ///
    /// Note: RustRover doesn't seem to recognize this method in some cases. You can also run the
    /// [`RoadsterApp`] using [`crate::app::run`] directly instead.
    pub async fn run(self) -> RoadsterResult<()> {
        run::run(self).await?;

        Ok(())
    }
}

impl<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
    #[cfg(not(feature = "cli"))] Cli: 'static,
> Default for RoadsterAppBuilder<S, Cli>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
    #[cfg(not(feature = "cli"))] Cli: 'static,
> RoadsterAppBuilder<S, Cli>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    pub fn new() -> Self {
        Self {
            inner: Inner::new(),
            async_config_sources: Default::default(),
            #[cfg(feature = "db-sea-orm")]
            sea_orm_migrator: Default::default(),
            #[cfg(feature = "db-diesel")]
            diesel_migrator: Default::default(),
            #[cfg(feature = "db-sql")]
            migrators: Default::default(),
            #[cfg(feature = "db-diesel-postgres-pool")]
            diesel_pg_connection_customizer: Default::default(),
            #[cfg(feature = "db-diesel-mysql-pool")]
            diesel_mysql_connection_customizer: Default::default(),
            #[cfg(feature = "db-diesel-sqlite-pool")]
            diesel_sqlite_connection_customizer: Default::default(),
            #[cfg(feature = "db-diesel-postgres-pool-async")]
            diesel_pg_async_connection_customizer: Default::default(),
            #[cfg(feature = "db-diesel-mysql-pool-async")]
            diesel_mysql_async_connection_customizer: Default::default(),
            lifecycle_handlers: Default::default(),
            services: Default::default(),
        }
    }

    /// Add an async config source ([`AsyncSource`]). Useful to load configs/secrets from an
    /// external service, e.g., AWS or GCS secrets manager services.
    pub fn add_async_config_source(mut self, source: impl AsyncSource + Send + 'static) -> Self {
        self.async_config_sources.push(Box::new(source));
        self
    }

    /// Add an async config source ([`AsyncSource`]). Useful to load configs/secrets from an
    /// external service, e.g., AWS or GCS secrets manager services.
    pub fn add_async_config_source_provider(
        mut self,
        source_provider: impl 'static
        + Send
        + Sync
        + Fn(&Environment) -> RoadsterResult<Box<dyn AsyncSource + Send + Sync>>,
    ) -> Self {
        self.inner.add_async_config_source_provider(source_provider);
        self
    }

    /// Provide the logic to initialize tracing for the [`RoadsterApp`].
    pub fn tracing_initializer(
        mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.tracing_initializer(tracing_initializer);
        self
    }

    /// Provide the [`AppMetadata`] for the [`RoadsterApp`].
    pub fn metadata(mut self, metadata: AppMetadata) -> Self {
        self.inner.set_metadata(metadata);
        self
    }

    /// Provide the logic to build the [`AppMetadata`] for the [`RoadsterApp`].
    pub fn metadata_provider(
        mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) -> Self {
        self.inner.metadata_provider(metadata_provider);
        self
    }

    /// Provide the [`ConnectOptions`] for the [`RoadsterApp`].
    #[cfg(feature = "db-sea-orm")]
    pub fn sea_orm_conn_options(mut self, sea_orm_conn_options: ConnectOptions) -> Self {
        self.inner.sea_orm_conn_options(sea_orm_conn_options);
        self
    }

    /// Provide the logic to build the [`ConnectOptions`] for the [`RoadsterApp`].
    #[cfg(feature = "db-sea-orm")]
    pub fn sea_orm_conn_options_provider(
        mut self,
        sea_orm_conn_options_provider: impl 'static
        + Send
        + Sync
        + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) -> Self {
        self.inner
            .sea_orm_conn_options_provider(sea_orm_conn_options_provider);
        self
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    pub fn diesel_pg_connection_customizer(
        mut self,
        connection_customizer: impl 'static
        + r2d2::CustomizeConnection<
            crate::db::DieselPgConn,
            diesel::r2d2::Error,
        >,
    ) -> Self {
        self.diesel_pg_connection_customizer = Some(Box::new(connection_customizer));
        self
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    pub fn diesel_pg_connection_customizer_provider(
        mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselPgConn, diesel::r2d2::Error>>,
        >,
    ) -> Self {
        self.inner
            .diesel_pg_connection_customizer_provider(connection_customizer);
        self
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    pub fn diesel_mysql_connection_customizer(
        mut self,
        connection_customizer: impl 'static
        + r2d2::CustomizeConnection<
            crate::db::DieselMysqlConn,
            diesel::r2d2::Error,
        >,
    ) -> Self {
        self.diesel_mysql_connection_customizer = Some(Box::new(connection_customizer));
        self
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    pub fn diesel_mysql_connection_customizer_provider(
        mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselMysqlConn, diesel::r2d2::Error>>,
        >,
    ) -> Self {
        self.inner
            .diesel_mysql_connection_customizer_provider(connection_customizer);
        self
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    pub fn diesel_sqlite_connection_customizer(
        mut self,
        connection_customizer: impl 'static
        + r2d2::CustomizeConnection<
            crate::db::DieselSqliteConn,
            diesel::r2d2::Error,
        >,
    ) -> Self {
        self.diesel_sqlite_connection_customizer = Some(Box::new(connection_customizer));
        self
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    pub fn diesel_sqlite_connection_customizer_provider(
        mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<dyn r2d2::CustomizeConnection<crate::db::DieselSqliteConn, diesel::r2d2::Error>>,
        >,
    ) -> Self {
        self.inner
            .diesel_sqlite_connection_customizer_provider(connection_customizer);
        self
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    pub fn diesel_pg_async_connection_customizer(
        mut self,
        connection_customizer: impl 'static
        + bb8_8::CustomizeConnection<
            crate::db::DieselPgConnAsync,
            diesel_async::pooled_connection::PoolError,
        >,
    ) -> Self {
        self.diesel_pg_async_connection_customizer = Some(Box::new(connection_customizer));
        self
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    pub fn diesel_pg_async_connection_customizer_provider(
        mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<
                dyn bb8_8::CustomizeConnection<
                        crate::db::DieselPgConnAsync,
                        diesel_async::pooled_connection::PoolError,
                    >,
            >,
        >,
    ) -> Self {
        self.inner
            .diesel_pg_async_connection_customizer_provider(connection_customizer);
        self
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    pub fn diesel_mysql_async_connection_customizer(
        mut self,
        connection_customizer: impl 'static
        + bb8_8::CustomizeConnection<
            crate::db::DieselMysqlConnAsync,
            diesel_async::pooled_connection::PoolError,
        >,
    ) -> Self {
        self.diesel_mysql_async_connection_customizer = Some(Box::new(connection_customizer));
        self
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    pub fn diesel_mysql_async_connection_customizer_provider(
        mut self,
        connection_customizer: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        ) -> RoadsterResult<
            Box<
                dyn bb8_8::CustomizeConnection<
                        crate::db::DieselMysqlConnAsync,
                        diesel_async::pooled_connection::PoolError,
                    >,
            >,
        >,
    ) -> Self {
        self.inner
            .diesel_mysql_async_connection_customizer_provider(connection_customizer);
        self
    }

    /// Provide the [`sqlx::pool::PoolOptions`] for the [`RoadsterApp`] to use with the PG-backed
    /// worker service.
    #[cfg(feature = "worker-pg")]
    pub fn worker_pg_sqlx_pool_options(
        mut self,
        worker_pg_sqlx_pool_options: sqlx::pool::PoolOptions<sqlx::Postgres>,
    ) -> Self {
        self.inner
            .worker_pg_sqlx_pool_options(worker_pg_sqlx_pool_options);
        self
    }

    /// Provide the logic to build the [`sqlx::pool::PoolOptions`] for the [`RoadsterApp`] to use
    /// with the PG-backed worker service.
    #[cfg(feature = "worker-pg")]
    pub fn worker_pg_sqlx_pool_options_provider(
        mut self,
        worker_pg_sqlx_pool_options: impl 'static
        + Send
        + Sync
        + Fn(
            &AppConfig,
        )
            -> RoadsterResult<sqlx::pool::PoolOptions<sqlx::Postgres>>,
    ) -> Self {
        self.inner
            .worker_pg_sqlx_pool_options_provider(worker_pg_sqlx_pool_options);
        self
    }

    /// Provide the logic to build the custom state for the [`RoadsterApp`].
    pub fn state_provider(
        mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) -> Self {
        self.inner.state_provider(builder);
        self
    }

    /// Add the diesel migrator [`sea_orm_migration::MigratorTrait`] to run on app start up
    /// (if the `database.auto-migrate` config field is set to `true`)
    ///
    /// Note: SeaORM migrations expect all of the applied migrations to be available
    /// to the provided migrator, so only a single SeaORM migrator is allowed.
    #[cfg(feature = "db-sea-orm")]
    pub fn sea_orm_migrator(
        mut self,
        migrator: impl 'static + Sync + sea_orm_migration::MigratorTrait,
    ) -> Self {
        self.sea_orm_migrator = Some(Box::new(
            crate::db::migration::sea_orm::SeaOrmMigrator::new(migrator),
        ));
        self
    }

    /// Add the diesel migrator [`diesel::migration::MigrationSource`] to run on app start up
    /// (if the `database.auto-migrate` config field is set to `true`)
    ///
    /// Note: Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so only a single Diesel migrator is allowed.
    #[cfg(feature = "db-diesel")]
    pub fn diesel_migrator<C>(
        mut self,
        migrator: impl 'static + Send + Sync + diesel::migration::MigrationSource<C::Backend>,
    ) -> Self
    where
        C: 'static
            + diesel::connection::Connection
            + Send
            + diesel_migrations::MigrationHarness<C::Backend>,
    {
        self.diesel_migrator = Some(Box::new(
            crate::db::migration::diesel::DieselMigrator::<C>::new(migrator),
        ));
        self
    }

    /// Add a [`Migrator`] to run on app start up (if the `database.auto-migrate` config field is
    /// set to `true`).
    ///
    /// Note: SeaORM and Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so multiple SeaORM or Diesel migrators should not be provided
    /// via this method.
    #[cfg(feature = "db-sql")]
    pub fn add_migrator(mut self, migrator: impl Migrator<S> + 'static) -> Self {
        self.migrators.push(Box::new(migrator));
        self
    }

    /// Add a [`MigratorProvider`] that provides a [`Migrator`] to run on app start up
    /// (if the `database.auto-migrate` config field is set to `true`).
    ///
    /// This is useful compared to [`Self::add_migrator`] if the [`Migrator`] implementation
    /// needs access to the app state for any reason.
    ///
    /// Note: SeaORM and Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so multiple SeaORM or Diesel migrators should not be provided
    /// via this method.
    #[cfg(feature = "db-sql")]
    pub fn add_migrator_provider(
        mut self,
        migrator_provider: impl 'static + Send + Sync + Fn(&S) -> RoadsterResult<Box<dyn Migrator<S>>>,
    ) -> Self {
        self.inner.add_migrator_provider(migrator_provider);
        self
    }

    /// Add a [`AppLifecycleHandler`] for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple handlers.
    pub fn add_lifecycle_handler(
        mut self,
        lifecycle_handler: impl 'static + AppLifecycleHandler<RoadsterApp<S, Cli>, S>,
    ) -> Self {
        self.lifecycle_handlers.push(Box::new(lifecycle_handler));
        self
    }

    /// Provide the logic to register [`AppLifecycleHandler`]s for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple handlers in separate
    /// callbacks.
    pub fn add_lifecycle_handler_provider(
        mut self,
        lifecycle_handler_provider: impl 'static
        + Send
        + Sync
        + Fn(
            &mut LifecycleHandlerRegistry<RoadsterApp<S, Cli>, S>,
            &S,
        ) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .lifecycle_handler_providers
            .push(Box::new(lifecycle_handler_provider));
        self
    }

    /// Add a [`HealthCheck`] for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple health checks.
    pub fn add_health_check(mut self, health_check: impl 'static + HealthCheck) -> Self {
        self.inner.add_health_check(health_check);
        self
    }

    /// Provide the logic to register [`HealthCheck`]s for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple health checks in separate
    /// callbacks.
    pub fn add_health_check_provider(
        mut self,
        health_check_provider: impl 'static
        + Send
        + Sync
        + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.add_health_check_provider(health_check_provider);
        self
    }

    /// Add a [`AppService`] for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple services.
    pub fn add_service(
        mut self,
        service: impl 'static + AppService<RoadsterApp<S, Cli>, S>,
    ) -> Self {
        self.services.push(Box::new(service));
        self
    }

    /// Provide the logic to register [`AppService`]s for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple services in separate
    /// callbacks.
    pub fn add_service_provider(
        mut self,
        service_provider: impl 'static
        + Send
        + Sync
        + for<'a> Fn(
            &'a mut ServiceRegistry<RoadsterApp<S, Cli>, S>,
            &'a S,
        ) -> Pin<
            Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>,
        >,
    ) -> Self {
        self.inner
            .service_providers
            .push(Box::new(service_provider));
        self
    }

    /// Provide a custom signal to listen for in order to shutdown the [`RoadsterApp`].
    pub fn graceful_shutdown_signal_provider(
        mut self,
        graceful_shutdown_signal_provider: impl 'static
        + Send
        + Sync
        + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) -> Self {
        self.inner
            .provide_graceful_shutdown_signal(graceful_shutdown_signal_provider);
        self
    }

    /// Build the [`RoadsterApp`] from this [`RoadsterAppBuilder`].
    pub fn build(self) -> RoadsterApp<S, Cli> {
        RoadsterApp {
            inner: self.inner,
            async_config_sources: Mutex::new(self.async_config_sources),
            #[cfg(feature = "db-sea-orm")]
            sea_orm_migrator: Mutex::new(self.sea_orm_migrator),
            #[cfg(feature = "db-diesel")]
            diesel_migrator: Mutex::new(self.diesel_migrator),
            #[cfg(feature = "db-sql")]
            migrators: Mutex::new(self.migrators),
            #[cfg(feature = "db-diesel-postgres-pool")]
            diesel_pg_connection_customizer: Mutex::new(self.diesel_pg_connection_customizer),
            #[cfg(feature = "db-diesel-mysql-pool")]
            diesel_mysql_connection_customizer: Mutex::new(self.diesel_mysql_connection_customizer),
            #[cfg(feature = "db-diesel-sqlite-pool")]
            diesel_sqlite_connection_customizer: Mutex::new(
                self.diesel_sqlite_connection_customizer,
            ),
            #[cfg(feature = "db-diesel-postgres-pool-async")]
            diesel_pg_async_connection_customizer: Mutex::new(
                self.diesel_pg_async_connection_customizer,
            ),
            #[cfg(feature = "db-diesel-mysql-pool-async")]
            diesel_mysql_async_connection_customizer: Mutex::new(
                self.diesel_mysql_async_connection_customizer,
            ),
            lifecycle_handlers: Mutex::new(self.lifecycle_handlers),
            services: Mutex::new(self.services),
        }
    }
}

#[async_trait]
impl<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
    #[cfg(not(feature = "cli"))] Cli: 'static,
> App<S> for RoadsterApp<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    type Cli = Cli;

    fn async_config_sources(
        &self,
        environment: &Environment,
    ) -> RoadsterResult<Vec<Box<dyn AsyncSource + Send + Sync>>> {
        let mut async_config_sources = self
            .async_config_sources
            .lock()
            .map_err(crate::error::Error::from)?;

        let mut sources: Vec<Box<dyn AsyncSource + Send + Sync>> = Default::default();
        for source in async_config_sources.drain(..) {
            sources.push(source);
        }

        for provider in self.inner.async_config_source_providers.iter() {
            let source = provider(environment)?;
            sources.push(source);
        }

        Ok(sources)
    }

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.get_metadata(config)
    }

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        self.inner.sea_orm_connection_options(config)
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    fn diesel_pg_connection_customizer(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselPgConn, diesel::r2d2::Error>>,
    > {
        let mut connection_customizer = self
            .diesel_pg_connection_customizer
            .lock()
            .map_err(crate::error::Error::from)?;

        if let Some(connection_customizer) = connection_customizer.take() {
            return Ok(connection_customizer);
        }

        if let Some(connection_customizer_provider) =
            self.inner.diesel_pg_connection_customizer_provider.as_ref()
        {
            return connection_customizer_provider(config);
        };

        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    fn diesel_mysql_connection_customizer(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselMysqlConn, diesel::r2d2::Error>>,
    > {
        let mut connection_customizer = self
            .diesel_mysql_connection_customizer
            .lock()
            .map_err(crate::error::Error::from)?;

        if let Some(connection_customizer) = connection_customizer.take() {
            return Ok(connection_customizer);
        }

        if let Some(connection_customizer_provider) = self
            .inner
            .diesel_mysql_connection_customizer_provider
            .as_ref()
        {
            return connection_customizer_provider(config);
        };

        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    fn diesel_sqlite_connection_customizer(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<
        Box<dyn r2d2::CustomizeConnection<crate::db::DieselSqliteConn, diesel::r2d2::Error>>,
    > {
        let mut connection_customizer = self
            .diesel_sqlite_connection_customizer
            .lock()
            .map_err(crate::error::Error::from)?;

        if let Some(connection_customizer) = connection_customizer.take() {
            return Ok(connection_customizer);
        }

        if let Some(connection_customizer_provider) = self
            .inner
            .diesel_sqlite_connection_customizer_provider
            .as_ref()
        {
            return connection_customizer_provider(config);
        };

        Ok(Box::new(r2d2::NopConnectionCustomizer))
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    fn diesel_pg_async_connection_customizer(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<
        Box<
            dyn bb8_8::CustomizeConnection<
                    crate::db::DieselPgConnAsync,
                    diesel_async::pooled_connection::PoolError,
                >,
        >,
    > {
        let mut connection_customizer = self
            .diesel_pg_async_connection_customizer
            .lock()
            .map_err(crate::error::Error::from)?;

        if let Some(connection_customizer) = connection_customizer.take() {
            return Ok(connection_customizer);
        }

        if let Some(connection_customizer_provider) = self
            .inner
            .diesel_pg_async_connection_customizer_provider
            .as_ref()
        {
            return connection_customizer_provider(config);
        };

        Ok(Box::new(Empty))
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    fn diesel_mysql_async_connection_customizer(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<
        Box<
            dyn bb8_8::CustomizeConnection<
                    crate::db::DieselMysqlConnAsync,
                    diesel_async::pooled_connection::PoolError,
                >,
        >,
    > {
        let mut connection_customizer = self
            .diesel_mysql_async_connection_customizer
            .lock()
            .map_err(crate::error::Error::from)?;

        if let Some(connection_customizer) = connection_customizer.take() {
            return Ok(connection_customizer);
        }

        if let Some(connection_customizer_provider) = self
            .inner
            .diesel_mysql_async_connection_customizer_provider
            .as_ref()
        {
            return connection_customizer_provider(config);
        };

        Ok(Box::new(Empty))
    }

    #[cfg(feature = "worker-pg")]
    fn worker_pg_sqlx_pool_options(
        &self,
        config: &AppConfig,
    ) -> RoadsterResult<sqlx::pool::PoolOptions<sqlx::Postgres>> {
        self.inner.build_worker_pg_sqlx_pool_options(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.provide_state(context).await
    }

    #[cfg(feature = "db-sql")]
    fn migrators(&self, state: &S) -> RoadsterResult<Vec<Box<dyn Migrator<S>>>> {
        let mut result = Vec::new();

        #[cfg(feature = "db-sea-orm")]
        {
            let mut sea_orm_migrator = self
                .sea_orm_migrator
                .lock()
                .map_err(crate::error::Error::from)?;
            if let Some(sea_orm_migrator) = sea_orm_migrator.take() {
                result.push(sea_orm_migrator);
            }
        }

        #[cfg(feature = "db-diesel")]
        {
            let mut diesel_migrator = self
                .diesel_migrator
                .lock()
                .map_err(crate::error::Error::from)?;
            if let Some(diesel_migrator) = diesel_migrator.take() {
                result.push(diesel_migrator);
            }
        }

        let mut migrators = self.migrators.lock().map_err(crate::error::Error::from)?;
        for migrator in migrators.drain(..) {
            result.push(migrator);
        }

        for migrator_provider in self.inner.migrator_providers.iter() {
            result.push(migrator_provider(state)?);
        }

        Ok(result)
    }

    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        {
            let mut lifecycle_handlers = self
                .lifecycle_handlers
                .lock()
                .map_err(crate::error::Error::from)?;
            for lifecycle_handler in lifecycle_handlers.drain(..) {
                registry.register_boxed(lifecycle_handler)?;
            }
        }

        for provider in self.inner.lifecycle_handler_providers.iter() {
            provider(registry, state)?;
        }
        Ok(())
    }

    async fn health_checks(
        &self,
        registry: &mut HealthCheckRegistry,
        state: &S,
    ) -> RoadsterResult<()> {
        self.inner.health_checks(registry, state).await
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        {
            let mut services = self.services.lock().map_err(crate::error::Error::from)?;
            for service in services.drain(..) {
                registry.register_boxed(service)?;
            }
        }

        for provider in self.inner.service_providers.iter() {
            provider(registry, state).await?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(self: Arc<Self>, state: &S) {
        self.inner.graceful_shutdown_signal(state).await
    }
}
