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

pub use prepare::{prepare, PrepareOptions, PreparedApp, PreparedAppCli, PreparedAppWithoutCli};
pub use run::{run, run_prepared};
#[cfg(feature = "testing")]
pub use test::{run_test, run_test_with_result, test_state};

#[cfg(all(test, feature = "cli"))]
use crate::api::cli::MockTestCli;
#[cfg(feature = "cli")]
use crate::api::cli::RunCommand;
use crate::app::metadata::AppMetadata;
use crate::config::environment::Environment;
use crate::config::AppConfig;
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
use std::future;
use std::sync::Arc;

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
