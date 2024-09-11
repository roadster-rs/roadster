#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app;
use crate::app::context::AppContext;
use crate::app::metadata::AppMetadata;
use crate::app::App;
use crate::config::app_config::AppConfig;
use crate::error::RoadsterResult;
use crate::health_check::registry::HealthCheckRegistry;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use crate::service::registry::ServiceRegistry;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::extract::FromRef;
use cfg_if::cfg_if;
#[cfg(feature = "db-sql")]
use sea_orm::ConnectOptions;
#[cfg(feature = "db-sql")]
use sea_orm_migration::MigratorTrait;
use std::future;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type StateBuilder<S> = dyn Send + Sync + Fn(AppContext) -> RoadsterResult<S>;
type TracingInitializer = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>;
type MetadataProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>;
#[cfg(feature = "db-sql")]
type DbConnOptionsProvider = dyn Send + Sync + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>;
type LifecycleHandlerProviders<A, S> =
    Vec<Box<dyn Send + Sync + Fn(&mut LifecycleHandlerRegistry<A, S>, &S) -> RoadsterResult<()>>>;
type HealthCheckProviders<S> =
    Vec<Box<dyn Send + Sync + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>>>;
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

/// Inner state shared between [`RoadsterApp`] and [`RoadsterAppBuilder`] that doesn't need
/// to modify type parameters depending on which features are enabled.
struct InnerCommon<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state_provider: Option<Box<StateBuilder<S>>>,
    tracing_initializer: Option<Box<TracingInitializer>>,
    metadata_provider: Option<Box<MetadataProvider>>,
    #[cfg(feature = "db-sql")]
    db_conn_options_provider: Option<Box<DbConnOptionsProvider>>,
    health_check_providers: HealthCheckProviders<S>,
    graceful_shutdown_signal_provider: GracefulShutdownSignalProvider<S>,
}

impl<S> InnerCommon<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new() -> Self {
        Self {
            state_provider: None,
            tracing_initializer: None,
            metadata_provider: None,
            #[cfg(feature = "db-sql")]
            db_conn_options_provider: None,
            health_check_providers: Default::default(),
            graceful_shutdown_signal_provider: None,
        }
    }

    fn tracing_initializer(
        &mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) {
        self.tracing_initializer = Some(Box::new(tracing_initializer));
    }

    fn metadata_provider(
        &mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) {
        self.metadata_provider = Some(Box::new(metadata_provider));
    }

    #[cfg(feature = "db-sql")]
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
            crate::tracing::init_tracing(config, &self.metadata(config)?)
        }
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        if let Some(metadata_provider) = self.metadata_provider.as_ref() {
            metadata_provider(config)
        } else {
            Ok(Default::default())
        }
    }

    #[cfg(feature = "db-sql")]
    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        if let Some(db_conn_options_provider) = self.db_conn_options_provider.as_ref() {
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

// This conditional compilation block is necessary because the type parameters for RoadsterApp
// are different depending on whether the `cli` and `db-sql` features are enabled. I haven't
// been able to find a better way to do this. We may need to refactor the `App` trait itself (and
// a bunch of other stuff) in order to improve this.
// todo: This conditional compilation block is gnarly. Is there a better way?
cfg_if! {
if #[cfg(all(feature = "cli", feature="db-sql"))] {

/// Inner state shared between both the [`RoadsterApp`] and [`RoadsterAppBuilder`].
struct Inner<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync + 'static,
    M: MigratorTrait + Send + Sync + 'static,
{
    common: InnerCommon<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S, Cli, M>, S>,
    service_providers: ServiceProviders<RoadsterApp<S, Cli, M>, S>,
}

pub struct RoadsterApp<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync + 'static,
    M: MigratorTrait + Send + Sync + 'static,
{
    inner: Inner<S, Cli, M>,
}

pub struct RoadsterAppBuilder<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync + 'static,
    M: MigratorTrait + Send + Sync + 'static,
{
    inner: Inner<S, Cli, M>,
}

impl<S, Cli, M> RoadsterApp<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
    M: 'static + MigratorTrait + Send + Sync,
{
    /// Create a new [`RoadsterAppBuilder`] to use to build the [`RoadsterApp`].
    pub fn builder() -> RoadsterAppBuilder<S, Cli, M> {
        RoadsterAppBuilder {
            inner: Inner {
                common: InnerCommon::new(),
                lifecycle_handler_providers: Default::default(),
                service_providers: Default::default(),
            },
        }
    }

    /// Utility method to run the [`RoadsterApp`].
    pub async fn run(self) -> RoadsterResult<()> {
        app::run(self).await?;

        Ok(())
    }
}

impl<S, Cli, M> RoadsterAppBuilder<S, Cli, M>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
    Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
    M: 'static + MigratorTrait + Send + Sync,
{
    /// Provide the logic to initialize tracing for the [`RoadsterApp`].
    pub fn tracing_initializer(
        mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.common.tracing_initializer(tracing_initializer);
        self
    }

    /// Provide the logic to build the [`AppMetadata`] for the [`RoadsterApp`].
    pub fn metadata_provider(
        mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) -> Self {
        self.inner.common.metadata_provider(metadata_provider);
        self
    }

    /// Provide the logic to build the [`ConnectOptions`] for the [`RoadsterApp`].
    pub fn db_conn_options_provider(
        mut self,
        db_conn_options_provider: impl 'static
            + Send
            + Sync
            + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) -> Self {
        self.inner
            .common
            .db_conn_options_provider(db_conn_options_provider);
        self
    }

    /// Provide the logic to build the custom state for the [`RoadsterApp`].
    pub fn state_provider(
        mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) -> Self {
        self.inner.common.state_provider(builder);
        self
    }

    /// Provide the logic to register [`crate::lifecycle::AppLifecycleHandler`]s for the
    /// [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple handlers in separate
    /// callbacks.
    pub fn lifecycle_handler_provider(
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

    /// Provide the logic to register [`crate::health_check::HealthCheck`]s for the
    /// [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple health checks in separate
    /// callbacks.
    pub fn health_check_provider(
        mut self,
        health_check_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .common
            .add_health_check_provider(health_check_provider);
        self
    }

    /// Provide the logic to register [`crate::service::AppService`]s for the
    /// [`RoadsterApp`].
    ///
    /// This method can be called multiple times to register multiple services in separate
    /// callbacks.
    pub fn service_provider(
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
    pub fn provide_graceful_shutdown_signal(
        mut self,
        graceful_shutdown_signal_provider: impl 'static
            + Send
            + Sync
            + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) -> Self {
        self.inner
            .common
            .provide_graceful_shutdown_signal(graceful_shutdown_signal_provider);
        self
    }

    /// Build the [`RoadsterApp`] from this [`RoadsterAppBuilder`].
    pub fn build(self) -> RoadsterApp<S, Cli, M> {
        RoadsterApp { inner: self.inner }
    }
}

#[async_trait]
impl<S, Cli, M> App<S> for RoadsterApp<S, Cli, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli, M>, S> + Send + Sync,
    M: MigratorTrait + Send + Sync,
{
    type Cli = Cli;
    type M = M;

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.common.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.common.metadata(config)
    }

    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        self.inner.common.db_connection_options(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.common.provide_state(context).await
    }

    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
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
        self.inner.common.health_checks(registry, state).await
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        for provider in self.inner.service_providers.iter() {
            provider(registry, state).await?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(self: Arc<Self>, state: &S) {
        self.inner.common.graceful_shutdown_signal(state).await
    }
}

} else if #[cfg(feature = "cli")] {

struct Inner<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync + 'static,
{
    common: InnerCommon<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S, Cli>, S>,
    service_providers: ServiceProviders<RoadsterApp<S, Cli>, S>,
}

pub struct RoadsterApp<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync + 'static,
{
    inner: Inner<S, Cli>,
}

pub struct RoadsterAppBuilder<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync + 'static,
{
    inner: Inner<S, Cli>,
}

impl<S, Cli> RoadsterApp<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
{
    pub fn builder() -> RoadsterAppBuilder<S, Cli> {
        RoadsterAppBuilder {
            inner: Inner {
                common: InnerCommon::new(),
                lifecycle_handler_providers: Default::default(),
                service_providers: Default::default(),
            },
        }
    }

    pub async fn run(self) -> RoadsterResult<()> {
        app::run(self).await?;

        Ok(())
    }
}

impl<S, Cli> RoadsterAppBuilder<S, Cli>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
    Cli: 'static + clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
{
    pub fn tracing_initializer(
        mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.common.tracing_initializer(tracing_initializer);
        self
    }

    pub fn metadata_provider(
        mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) -> Self {
        self.inner.common.metadata_provider(metadata_provider);
        self
    }

    pub fn state_provider(
        mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) -> Self {
        self.inner.common.state_provider(builder);
        self
    }

    pub fn lifecycle_handler_provider(
        mut self,
        lifecycle_handler_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut LifecycleHandlerRegistry<RoadsterApp<S, Cli>, S>, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .lifecycle_handler_providers
            .push(Box::new(lifecycle_handler_provider));
        self
    }

    pub fn health_check_provider(
        mut self,
        health_check_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .common
            .add_health_check_provider(health_check_provider);
        self
    }

    pub fn service_provider(
        mut self,
        service_provider: impl 'static
            + Send
            + Sync
            + for<'a> Fn(
                &'a mut ServiceRegistry<RoadsterApp<S, Cli>, S>,
                &'a S,
            ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
    ) -> Self {
        self.inner
            .service_providers
            .push(Box::new(service_provider));
        self
    }

    pub fn provide_graceful_shutdown_signal(
        mut self,
        graceful_shutdown_signal_provider: impl 'static
            + Send
            + Sync
            + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) -> Self {
        self.inner
            .common
            .provide_graceful_shutdown_signal(graceful_shutdown_signal_provider);
        self
    }

    pub fn build(self) -> RoadsterApp<S, Cli> {
        RoadsterApp { inner: self.inner }
    }
}

#[async_trait]
impl<S, Cli> App<S> for RoadsterApp<S, Cli>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Cli: clap::Args + RunCommand<RoadsterApp<S, Cli>, S> + Send + Sync,
{
    type Cli = Cli;

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.common.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.common.metadata(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.common.provide_state(context).await
    }

    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
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
        self.inner.common.health_checks(registry, state).await
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        for provider in self.inner.service_providers.iter() {
            provider(registry, state).await?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(self: Arc<Self>, state: &S) {
        self.inner.common.graceful_shutdown_signal(state).await
    }
}


} else if #[cfg(feature = "db-sql")] {

struct Inner<S, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: MigratorTrait + Send + Sync + 'static,
{
    common: InnerCommon<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S, M>, S>,
    service_providers: ServiceProviders<RoadsterApp<S, M>, S>,
}

pub struct RoadsterApp<S, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: MigratorTrait + Send + Sync + 'static,
{
    inner: Inner<S, M>,
}

pub struct RoadsterAppBuilder<S, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: MigratorTrait + Send + Sync + 'static,
{
    inner: Inner<S, M>,
}

impl<S, M> RoadsterApp<S, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: 'static + MigratorTrait + Send + Sync,
{
    pub fn builder() -> RoadsterAppBuilder<S, M> {
        RoadsterAppBuilder {
            inner: Inner {
                common: InnerCommon::new(),
                lifecycle_handler_providers: Default::default(),
                service_providers: Default::default(),
            },
        }
    }

    pub async fn run(self) -> RoadsterResult<()> {
        app::run(self).await?;

        Ok(())
    }
}

impl<S, M> RoadsterAppBuilder<S, M>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
    M: 'static + MigratorTrait + Send + Sync,
{
    pub fn tracing_initializer(
        mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.common.tracing_initializer(tracing_initializer);
        self
    }

    pub fn metadata_provider(
        mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) -> Self {
        self.inner.common.metadata_provider(metadata_provider);
        self
    }

    pub fn db_conn_options_provider(
        mut self,
        db_conn_options_provider: impl 'static
            + Send
            + Sync
            + Fn(&AppConfig) -> RoadsterResult<ConnectOptions>,
    ) -> Self {
        self.inner
            .common
            .db_conn_options_provider(db_conn_options_provider);
        self
    }

    pub fn state_provider(
        mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) -> Self {
        self.inner.common.state_provider(builder);
        self
    }

    pub fn lifecycle_handler_provider(
        mut self,
        lifecycle_handler_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut LifecycleHandlerRegistry<RoadsterApp<S, M>, S>, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .lifecycle_handler_providers
            .push(Box::new(lifecycle_handler_provider));
        self
    }

    pub fn health_check_provider(
        mut self,
        health_check_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .common
            .add_health_check_provider(health_check_provider);
        self
    }

    pub fn service_provider(
        mut self,
        service_provider: impl 'static
            + Send
            + Sync
            + for<'a> Fn(
                &'a mut ServiceRegistry<RoadsterApp<S, M>, S>,
                &'a S,
            ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
    ) -> Self {
        self.inner
            .service_providers
            .push(Box::new(service_provider));
        self
    }

    pub fn provide_graceful_shutdown_signal(
        mut self,
        graceful_shutdown_signal_provider: impl 'static
            + Send
            + Sync
            + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) -> Self {
        self.inner
            .common
            .provide_graceful_shutdown_signal(graceful_shutdown_signal_provider);
        self
    }

    pub fn build(self) -> RoadsterApp<S, M> {
        RoadsterApp { inner: self.inner }
    }
}

#[async_trait]
impl<S, M> App<S> for RoadsterApp<S, M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: MigratorTrait + Send + Sync,
{
    #[cfg(feature = "cli")]
    type Cli = Cli;
    #[cfg(feature = "db-sql")]
    type M = M;

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.common.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.common.metadata(config)
    }

    fn db_connection_options(&self, config: &AppConfig) -> RoadsterResult<ConnectOptions> {
        self.inner.common.db_connection_options(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.common.provide_state(context).await
    }

    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
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
        self.inner.common.health_checks(registry, state).await
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        for provider in self.inner.service_providers.iter() {
            provider(registry, state).await?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(self: Arc<Self>, state: &S) {
        self.inner.common.graceful_shutdown_signal(state).await
    }
}

} else {

struct Inner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    common: InnerCommon<S>,
    lifecycle_handler_providers: LifecycleHandlerProviders<RoadsterApp<S>, S>,
    service_providers: ServiceProviders<RoadsterApp<S>, S>,
}

pub struct RoadsterApp<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S>,
}

pub struct RoadsterAppBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Inner<S>,
}

impl<S> RoadsterApp<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub fn builder() -> RoadsterAppBuilder<S> {
        RoadsterAppBuilder {
            inner: Inner {
                common: InnerCommon::new(),
                lifecycle_handler_providers: Default::default(),
                service_providers: Default::default(),
            },
        }
    }

    pub async fn run(self) -> RoadsterResult<()> {
        app::run(self).await?;

        Ok(())
    }
}

impl<S> RoadsterAppBuilder<S>
where
    S: 'static + Clone + Send + Sync,
    AppContext: FromRef<S>,
{
    pub fn tracing_initializer(
        mut self,
        tracing_initializer: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<()>,
    ) -> Self {
        self.inner.common.tracing_initializer(tracing_initializer);
        self
    }

    pub fn metadata_provider(
        mut self,
        metadata_provider: impl 'static + Send + Sync + Fn(&AppConfig) -> RoadsterResult<AppMetadata>,
    ) -> Self {
        self.inner.common.metadata_provider(metadata_provider);
        self
    }

    pub fn state_provider(
        mut self,
        builder: impl 'static + Send + Sync + Fn(AppContext) -> RoadsterResult<S>,
    ) -> Self {
        self.inner.common.state_provider(builder);
        self
    }

    pub fn lifecycle_handler_provider(
        mut self,
        lifecycle_handler_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut LifecycleHandlerRegistry<RoadsterApp<S>, S>, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .lifecycle_handler_providers
            .push(Box::new(lifecycle_handler_provider));
        self
    }

    pub fn health_check_provider(
        mut self,
        health_check_provider: impl 'static
            + Send
            + Sync
            + Fn(&mut HealthCheckRegistry, &S) -> RoadsterResult<()>,
    ) -> Self {
        self.inner
            .common
            .add_health_check_provider(health_check_provider);
        self
    }

    pub fn service_provider(
        mut self,
        service_provider: impl 'static
            + Send
            + Sync
            + for<'a> Fn(
                &'a mut ServiceRegistry<RoadsterApp<S>, S>,
                &'a S,
            ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
    ) -> Self {
        self.inner
            .service_providers
            .push(Box::new(service_provider));
        self
    }

    pub fn provide_graceful_shutdown_signal(
        mut self,
        graceful_shutdown_signal_provider: impl 'static
            + Send
            + Sync
            + Fn(&S) -> Pin<Box<dyn Send + Future<Output = ()>>>,
    ) -> Self {
        self.inner
            .common
            .provide_graceful_shutdown_signal(graceful_shutdown_signal_provider);
        self
    }

    pub fn build(self) -> RoadsterApp<S> {
        RoadsterApp { inner: self.inner }
    }
}

#[async_trait]
impl<S> App<S> for RoadsterApp<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{

    fn init_tracing(&self, config: &AppConfig) -> RoadsterResult<()> {
        self.inner.common.init_tracing(config)
    }

    fn metadata(&self, config: &AppConfig) -> RoadsterResult<AppMetadata> {
        self.inner.common.metadata(config)
    }

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<S> {
        self.inner.common.provide_state(context).await
    }

    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
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
        self.inner.common.health_checks(registry, state).await
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, S>,
        state: &S,
    ) -> RoadsterResult<()> {
        for provider in self.inner.service_providers.iter() {
            provider(registry, state).await?;
        }
        Ok(())
    }

    async fn graceful_shutdown_signal(self: Arc<Self>, state: &S) {
        self.inner.common.graceful_shutdown_signal(state).await
    }
}

}
}
