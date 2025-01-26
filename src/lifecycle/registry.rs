use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::lifecycle::default::default_lifecycle_handlers;
use crate::lifecycle::AppLifecycleHandler;
use anyhow::anyhow;
use axum_core::extract::FromRef;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::ops::Deref;
use tracing::info;

/// Registry for the app's [`AppLifecycleHandler`]s.
///
/// # Examples
#[cfg_attr(
    feature = "default",
    doc = r##"
```rust
# use async_trait::async_trait;
# use clap::Parser;
# use sea_orm_migration::{MigrationTrait, MigratorTrait};
# use tokio_util::sync::CancellationToken;
# use roadster::api::cli::RunCommand;
# use roadster::app::context::AppContext;
# use roadster::error::RoadsterResult;
# use roadster::service::function::service::FunctionService;
# use roadster::service::registry::ServiceRegistry;
# use roadster::app::App as RoadsterApp;
# use roadster::lifecycle::AppLifecycleHandler;
# use roadster::lifecycle::registry::LifecycleHandlerRegistry;
#
# #[derive(Debug, Parser)]
# #[command(version, about)]
# pub struct AppCli {}
#
# #[async_trait]
# impl RunCommand<App, AppContext> for AppCli {
#     #[allow(clippy::disallowed_types)]
#     async fn run(
#         &self,
#         _app: &App,
#         _cli: &AppCli,
#         _context: &AppContext,
#     ) -> RoadsterResult<bool> {
#         Ok(false)
#     }
# }
# pub struct Migrator;
#
# #[async_trait::async_trait]
# impl MigratorTrait for Migrator {
#     fn migrations() -> Vec<Box<dyn MigrationTrait>> {
#         Default::default()
#     }
# }
#
pub struct ExampleLifecycleHandler;

impl AppLifecycleHandler<App, AppContext> for ExampleLifecycleHandler {
    fn name(&self) -> String {
        "example".to_string()
    }
}

pub struct App;

#[async_trait]
impl RoadsterApp<AppContext> for App {
#     type Cli = AppCli;
#     type M = Migrator;
#
#     async fn provide_state(&self, _context: AppContext) -> RoadsterResult<AppContext> {
#         unimplemented!()
#     }
    async fn lifecycle_handlers(
        &self,
        registry: &mut LifecycleHandlerRegistry<Self, AppContext>,
        _state: &AppContext,
    ) -> RoadsterResult<()> {
        registry.register(ExampleLifecycleHandler)?;
        Ok(())
    }
}
```
"##
)]

pub struct LifecycleHandlerRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + ?Sized + 'static,
{
    state: S,
    handlers: BTreeMap<String, Box<dyn AppLifecycleHandler<A, S>>>,
}

impl<A, S> LifecycleHandlerRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            handlers: default_lifecycle_handlers(state),
        }
    }

    /// Register a new [`AppLifecycleHandler`]. If the [`AppLifecycleHandler`] is not enabled
    /// (e.g., [`AppLifecycleHandler::enabled`] returns `false`), the [`AppLifecycleHandler`]
    /// will not be registered.
    pub fn register<H>(&mut self, handler: H) -> RoadsterResult<()>
    where
        H: AppLifecycleHandler<A, S> + 'static,
    {
        self.register_boxed(Box::new(handler))
    }

    pub(crate) fn register_boxed(
        &mut self,
        handler: Box<dyn AppLifecycleHandler<A, S>>,
    ) -> RoadsterResult<()> {
        let name = handler.name();

        if !handler.enabled(&self.state) {
            info!(name=%name, "Lifecycle handler is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering lifecycle handler");

        if self.handlers.insert(name.clone(), handler).is_some() {
            return Err(anyhow!("Handler `{}` was already registered", name).into());
        }

        Ok(())
    }

    /// Get the registered [`AppLifecycleHandler`]s, ordered by their
    /// [`AppLifecycleHandler::priority`].
    pub(crate) fn handlers(&self, state: &S) -> Vec<&dyn AppLifecycleHandler<A, S>> {
        self.handlers
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            .map(|handler| handler.deref())
            .collect_vec()
    }
}
