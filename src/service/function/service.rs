use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::service::AppService;
use async_trait::async_trait;
use axum::extract::FromRef;
use std::future::Future;
use std::marker::PhantomData;
use tokio_util::sync::CancellationToken;
use typed_builder::TypedBuilder;

/// A generic [AppService] to allow creating a service from an async function.
///
/// # Examples
#[cfg_attr(
    feature = "default",
    doc = r##"
```rust
# use async_trait::async_trait;
# use clap::Parser;
# use sea_orm_migration::{MigrationTrait, MigratorTrait};
use tokio_util::sync::CancellationToken;
# use roadster::api::cli::RunCommand;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
use roadster::service::registry::ServiceRegistry;
use roadster::app::App as RoadsterApp;
#
# #[derive(Debug, Parser)]
# #[command(version, about)]
# pub struct AppCli {}
#
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

async fn example_service(
    _state: AppContext,
    _cancel_token: CancellationToken,
) -> RoadsterResult<()> {
    // Service logic here
    todo!()
}

pub struct App;

#[async_trait]
impl RoadsterApp<AppContext> for App {
#     type Cli = AppCli;
#     type M = Migrator;
#
#     async fn provide_state(&self, _context: AppContext) -> RoadsterResult<AppContext> {
#         todo!()
#     }
    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, AppContext>,
        context: &AppContext,
    ) -> RoadsterResult<()> {
        let service = FunctionService::builder()
            .name("example".to_string())
            .enabled(true)
            .function(example_service)
            .build();

        registry.register_service(service)?;

        Ok(())
    }
}
```
"##
)]
#[derive(TypedBuilder)]
pub struct FunctionService<A, S, F, Fut>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    F: Send + Sync + Fn(S, CancellationToken) -> Fut,
    Fut: Send + Future<Output = RoadsterResult<()>>,
{
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    function: F,
    #[builder(default, setter(skip))]
    _app: PhantomData<A>,
    #[builder(default, setter(skip))]
    _state: PhantomData<S>,
}

#[async_trait]
impl<A, S, F, Fut> AppService<A, S> for FunctionService<A, S, F, Fut>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    F: Send + Sync + Fn(S, CancellationToken) -> Fut,
    Fut: Send + Future<Output = RoadsterResult<()>>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        self.enabled
            .unwrap_or(AppContext::from_ref(state).config().service.default_enable)
    }

    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        (self.function)(state.clone(), cancel_token).await
    }
}
