#[cfg(feature = "grpc")]
use crate::api::grpc::routes;
use crate::api::http;
use crate::app_state::CustomAppContext;
use crate::cli::AppCli;
use crate::service::example::example_service;
use crate::worker::example::ExampleWorker;
use async_trait::async_trait;
use migration::Migrator;
use roadster::app::context::AppContext;
use roadster::app::metadata::AppMetadata;
use roadster::app::App as RoadsterApp;
use roadster::config::app_config::AppConfig;
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
#[cfg(feature = "grpc")]
use roadster::service::grpc::service::GrpcService;
use roadster::service::http::service::HttpService;
use roadster::service::registry::ServiceRegistry;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;

const BASE: &str = "/api";

#[derive(Default)]
pub struct App;

#[async_trait]
impl RoadsterApp for App {
    type State = CustomAppContext;
    type Cli = AppCli;
    type M = Migrator;

    fn metadata(_config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(AppMetadata::builder()
            .version(env!("VERGEN_GIT_SHA").to_string())
            .build())
    }

    async fn with_state(_context: &AppContext) -> RoadsterResult<Self::State> {
        Ok(())
    }

    async fn services(
        registry: &mut ServiceRegistry<Self>,
        context: &AppContext<Self::State>,
    ) -> RoadsterResult<()> {
        registry
            .register_builder(
                HttpService::builder(Some(BASE), context).api_router(http::routes(BASE)),
            )
            .await?;

        registry
            .register_builder(
                SidekiqWorkerService::builder(context)
                    .await?
                    .register_app_worker(ExampleWorker::build(context))?,
            )
            .await?;

        registry.register_service(
            FunctionService::builder()
                .name("example".to_string())
                .function(example_service)
                .build(),
        )?;

        #[cfg(feature = "grpc")]
        registry.register_service(GrpcService::new(routes(context)?))?;

        Ok(())
    }
}
