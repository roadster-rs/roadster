#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app;
use crate::app::context::AppContext;
use crate::app::metadata::AppMetadata;
use crate::app::App;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::health::check::HealthCheck;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::lifecycle::AppLifecycleHandler;
use crate::service::registry::ServiceRegistry;
use crate::service::AppService;
use crate::util::empty::Empty;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
#[cfg(feature = "db-sea-orm")]
use sea_orm::ConnectOptions;
#[cfg(feature = "db-sea-orm")]
use sea_orm_migration::MigratorTrait;
use std::future;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

type StateBuilder<S> = dyn Send + Sync + Fn(AppContext) -> RoadsterResult<S>;
type TracingInitializer = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>;
type MetadataProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>;
#[cfg(feature = "db-sea-orm")]
type DbConnOptionsProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>;
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
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
    #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync = Empty,
    #[cfg(not(feature = "db-sea-orm"))] M: 'static = Empty,
> where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    state_provider: Option<Box<StateBuilder<S>>>,
    tracing_initializer: Option<Box<TracingInitializer>>,
    metadata: Option<AppMetadata>,
    metadata_provider: Option<Box<MetadataProvider>>,
    #[cfg(feature = "db-sea-orm")]
    db_conn_options: Option<ConnectOptions>,
    #[cfg(feature = "db-sea-orm")]
    db_conn_options_provider: Option<Box<DbConnOptionsProvider>>,
    health_checks: Vec<Arc<dyn HealthCheck>>,
    health_check_providers: HealthCheckProviders<S>,
    graceful_shutdown_signal_provider: GracefulShutdownSignalProvider<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S, Cli, M>, S>,
    service_providers: ServiceProviders<RoadsterApp<S, Cli, M>, S>,
}

impl<
        S,
        #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
        #[cfg(not(feature = "cli"))] Cli: 'static,
        #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync,
        #[cfg(not(feature = "db-sea-orm"))] M: 'static,
    > Inner<S, Cli, M>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    fn new() -> Self {
        Self {
            state_provider: None,
            tracing_initializer: None,
            metadata: None,
            metadata_provider: None,
            #[cfg(feature = "db-sea-orm")]
            db_conn_options: None,
            #[cfg(feature = "db-sea-orm")]
            db_conn_options_provider: None,
            health_checks: Default::default(),
            health_check_providers: Default::default(),
            graceful_shutdown_signal_provider: None,
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
    fn db_conn_options(&mut self, db_conn_options: ConnectOptions) {
        self.db_conn_options = Some(db_conn_options);
    }

    #[cfg(feature = "db-sea-orm")]
    fn db_conn_options_provider(
        &mut self,
        db_conn_options_provider: impl 'static
            + Send
            + Sync
            + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) {
        self.db_conn_options_provider = Some(Box::new(db_conn_options_provider));
    }

    fn state_provider(
        &mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) {
        self.state_provider = Some(Box::new(builder));
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
    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        if let Some(db_conn_options) = self.db_conn_options.as_ref() {
            Ok(db_conn_options.clone())
        } else if let Some(db_conn_options_provider) = self.db_conn_options_provider.as_ref() {
            db_conn_options_provider(config)
        } else {
            Ok(ConnectOptions::from(&config.database))
        }
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        let state_provider = self
            .state_provider
            .as_ref()
            .ok_or_else(|| anyhow!("State builder missing"))?;
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
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
    #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync = Empty,
    #[cfg(not(feature = "db-sea-orm"))] M: 'static = Empty,
> where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S, Cli, M>,
    // Interior mutability pattern -- this allows us to keep the handler reference as a
    // Box, which helps with single ownership and ensuring we only register a handler once.
    lifecycle_handlers: Mutex<LifecycleHandlers<RoadsterApp<S, Cli, M>, S>>,
    // Interior mutability pattern -- this allows us to keep the service reference as a
    // Box, which helps with single ownership and ensuring we only register a service once.
    services: Mutex<Services<RoadsterApp<S, Cli, M>, S>>,
}

pub struct RoadsterAppBuilder<
    S,
    #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync = Empty,
    #[cfg(not(feature = "cli"))] Cli: 'static = Empty,
    #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync = Empty,
    #[cfg(not(feature = "db-sea-orm"))] M: 'static = Empty,
> where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S, Cli, M>,
    lifecycle_handlers: LifecycleHandlers<RoadsterApp<S, Cli, M>, S>,
    services: Services<RoadsterApp<S, Cli, M>, S>,
}

impl<
        S,
        #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
        #[cfg(not(feature = "cli"))] Cli: 'static,
        #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync,
        #[cfg(not(feature = "db-sea-orm"))] M: 'static,
    > RoadsterApp<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    /// Create a new [`RoadsterAppBuilder`] to use to build the [`RoadsterApp`].
    pub fn builder() -> RoadsterAppBuilder<S, Cli, M> {
        RoadsterAppBuilder::new()
    }

    /// Utility method to run the [`RoadsterApp`].
    ///
    /// Note: RustRover doesn't seem to recognize this method in some cases. You can also run the
    /// [`RoadsterApp`] using [`app::run`] directly instead.
    pub async fn run(self) -> RoadsterResult<()> {
        app::run(self).await?;

        Ok(())
    }
}

impl<
        S,
        #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
        #[cfg(not(feature = "cli"))] Cli: 'static,
        #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync,
        #[cfg(not(feature = "db-sea-orm"))] M: 'static,
    > Default for RoadsterAppBuilder<S, Cli, M>
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
        #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
        #[cfg(not(feature = "cli"))] Cli: 'static,
        #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync,
        #[cfg(not(feature = "db-sea-orm"))] M: 'static,
    > RoadsterAppBuilder<S, Cli, M>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    pub fn new() -> Self {
        Self {
            inner: Inner::new(),
            lifecycle_handlers: Default::default(),
            services: Default::default(),
        }
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
    pub fn db_conn_options(mut self, db_conn_options: ConnectOptions) -> Self {
        self.inner.db_conn_options(db_conn_options);
        self
    }

    /// Provide the logic to build the [`ConnectOptions`] for the [`RoadsterApp`].
    #[cfg(feature = "db-sea-orm")]
    pub fn db_conn_options_provider(
        mut self,
        db_conn_options_provider: impl 'static
            + Send
            + Sync
            + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) -> Self {
        self.inner
            .db_conn_options_provider(db_conn_options_provider);
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

    /// Add a [`AppLifecycleHandler`] for the [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple handlers.
    pub fn add_lifecycle_handler(
        mut self,
        lifecycle_handler: impl 'static + AppLifecycleHandler<RoadsterApp<S, Cli, M>, S>,
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
            + Fn(&mut LifecycleHandlerRegistry<RoadsterApp<S, Cli, M>, S>, &S) -> RoadsterResult<()>,
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
        service: impl 'static + AppService<RoadsterApp<S, Cli, M>, S>,
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
                &'a mut ServiceRegistry<RoadsterApp<S, Cli, M>, S>,
                &'a S,
            ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
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
    pub fn build(self) -> RoadsterApp<S, Cli, M> {
        RoadsterApp {
            inner: self.inner,
            lifecycle_handlers: Mutex::new(self.lifecycle_handlers),
            services: Mutex::new(self.services),
        }
    }
}

#[async_trait]
impl<
        S,
        #[cfg(feature = "cli")] Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
        #[cfg(not(feature = "cli"))] Cli: 'static,
        #[cfg(feature = "db-sea-orm")] M: 'static + MigratorTrait + Send + Sync,
        #[cfg(not(feature = "db-sea-orm"))] M: 'static,
    > App<S> for RoadsterApp<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    type Cli = Cli;
    type M = M;

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.get_metadata(config)
    }

    #[cfg(feature = "db-sea-orm")]
    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        self.inner.db_connection_options(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.provide_state(context).await
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
                .map_err(|err| anyhow!("Unable to lock lifecycle_handlers mutex: {err}"))?;
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
            let mut services = self
                .services
                .lock()
                .map_err(|err| anyhow!("Unable to lock services mutex: {err}"))?;
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
