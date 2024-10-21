#[cfg(feature = "grpc")]
use crate::api::grpc::routes;
use crate::api::http;
use crate::api::http::hello_world_middleware_fn;
use crate::app_state::AppState;
use crate::cli::AppCli;
use crate::service::example::example_service;
use crate::worker::example::ExampleWorker;
use async_trait::async_trait;
use migration::Migrator;
use roadster::app::context::AppContext;
use roadster::app::metadata::AppMetadata;
use roadster::app::App as RoadsterApp;
use roadster::config::AppConfig;
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
#[cfg(feature = "grpc")]
use roadster::service::grpc::service::GrpcService;
use roadster::service::http::middleware::any::AnyMiddleware;
use roadster::service::http::service::HttpService;
use roadster::service::registry::ServiceRegistry;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;

const BASE: &str = "/api";

#[derive(Default)]
pub struct App;

#[async_trait]
impl RoadsterApp<AppState> for App {
    type Cli = AppCli;
    type M = Migrator;

    fn metadata(&self, _config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(AppMetadata::builder()
            .version(env!("VERGEN_GIT_SHA").to_string())
            .build())
    }

    async fn provide_state(&self, app_context: AppContext) -> RoadsterResult<AppState> {
        Ok(AppState { app_context })
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, AppState>,
        state: &AppState,
    ) -> RoadsterResult<()> {
        registry
            .register_builder(
                HttpService::builder(Some(BASE), state)
                    .api_router(http::routes(BASE))
                    .middleware(
                        AnyMiddleware::builder()
                            .name("hello-world")
                            .layer_provider(|_state| {
                                axum::middleware::from_fn(hello_world_middleware_fn)
                            })
                            .build(),
                    )?,
            )
            .await?;

        registry
            .register_builder(
                SidekiqWorkerService::builder(state)
                    .await?
                    .register_app_worker(ExampleWorker::build(state))?,
            )
            .await?;

        registry.register_service(
            FunctionService::builder()
                .name("example".to_string())
                .function(example_service)
                .build(),
        )?;

        #[cfg(feature = "grpc")]
        registry.register_service(GrpcService::new(routes(state)?))?;

        Ok(())
    }
}
