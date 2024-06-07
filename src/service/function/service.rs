use crate::app::App;
use crate::app_context::AppContext;
use crate::error::RoadsterResult;
use crate::service::AppService;
use async_trait::async_trait;
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
use roadster::app_context::AppContext;
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
# impl RunCommand<App> for AppCli {
#     #[allow(clippy::disallowed_types)]
#     async fn run(
#         &self,
#         _app: &App,
#         _cli: &AppCli,
#         _context: &AppContext<()>,
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
    _state: AppContext<()>,
    _cancel_token: CancellationToken,
) -> RoadsterResult<()> {
    // Service logic here
    Ok(())
}
pub struct App;
#[async_trait]
impl RoadsterApp for App {
#     type State = ();
#     type Cli = AppCli;
#     type M = Migrator;
#
#     async fn with_state(_context: &AppContext) -> RoadsterResult<Self::State> {
#         Ok(())
#     }
    async fn services(
        registry: &mut ServiceRegistry<Self>,
        context: &AppContext<Self::State>,
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
pub struct FunctionService<A, F, Fut>
where
    A: App + 'static,
    F: Send + Sync + Fn(AppContext<A::State>, CancellationToken) -> Fut,
    Fut: Send + Future<Output = RoadsterResult<()>>,
{
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    function: F,
    #[builder(default, setter(skip))]
    _app: PhantomData<A>,
}

#[async_trait]
impl<A, F, Fut> AppService<A> for FunctionService<A, F, Fut>
where
    A: App + 'static,
    F: Send + Sync + Fn(AppContext<A::State>, CancellationToken) -> Fut,
    Fut: Send + Future<Output = RoadsterResult<()>>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, context: &AppContext<A::State>) -> bool {
        self.enabled
            .unwrap_or(context.config().service.default_enable)
    }

    async fn run(
        self: Box<Self>,
        app_context: &AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        (self.function)(app_context.clone(), cancel_token).await
    }
}
