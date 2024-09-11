use app_builder::api::http;
use app_builder::app_state::AppState;
use app_builder::health_check::example::ExampleHealthCheck;
use app_builder::lifecycle::example::ExampleLifecycleHandler;
use app_builder::worker::example::ExampleWorker;
use app_builder::App;
use roadster::app::metadata::AppMetadata;
use roadster::app::RoadsterApp;
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;
use std::future;

const BASE: &str = "/api";

fn metadata() -> AppMetadata {
    AppMetadata::builder()
        .version(env!("VERGEN_GIT_SHA").to_string())
        .build()
}

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let custom_state = "custom".to_string();

    let builder = RoadsterApp::builder()
        .tracing_initializer(|config| roadster::tracing::init_tracing(config, &metadata()))
        .metadata_provider(move |_config| Ok(metadata()));

    #[cfg(feature = "db-sql")]
    let builder = builder
        .db_conn_options_provider(|config| Ok(sea_orm::ConnectOptions::from(&config.database)));

    let app: App = builder
        .state_provider(move |app_context| {
            Ok(AppState {
                app_context,
                custom_state: custom_state.clone(),
            })
        })
        .lifecycle_handler_provider(|registry, _state| {
            registry.register(ExampleLifecycleHandler)?;
            Ok(())
        })
        .health_check_provider(|registry, _state| {
            registry.register(ExampleHealthCheck)?;
            Ok(())
        })
        .service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        HttpService::builder(Some(BASE), state).api_router(http::routes(BASE)),
                    )
                    .await?;
                Ok(())
            })
        })
        .service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        SidekiqWorkerService::builder(state)
                            .await?
                            .register_app_worker(ExampleWorker::build(state))?,
                    )
                    .await?;
                Ok(())
            })
        })
        .provide_graceful_shutdown_signal(|_state| {
            Box::pin(async {
                let _output: () = future::pending().await;
            })
        })
        .build();

    app.run().await
}
