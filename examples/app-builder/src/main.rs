use app_builder::api::http;
use app_builder::app_state::AppState;
use app_builder::health::check::example::ExampleHealthCheck;
use app_builder::lifecycle::example::ExampleLifecycleHandler;
use app_builder::worker::example::ExampleWorker;
use app_builder::App;
use roadster::app::metadata::AppMetadata;
use roadster::app::RoadsterApp;
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
use roadster::service::http::service::HttpService;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;
use std::future;
use tokio_util::sync::CancellationToken;

const BASE: &str = "/api";

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let custom_state = "custom".to_string();

    let builder = RoadsterApp::builder()
        .tracing_initializer(|config| roadster::tracing::init_tracing(config, &metadata()));

    // Metadata can either be provided directly or via a provider callback. Note that the two
    // approaches are mutually exclusive, with the `metadata` method taking priority.
    let builder = builder
        .metadata(metadata())
        .metadata_provider(move |_config| Ok(metadata()));

    // Db connection options can either be provided directly or via a provider callback. Note that
    // the two approaches are mutually exclusive, with the `db_conn_options` method taking priority.
    #[cfg(feature = "db-sql")]
    let builder = {
        let mut db_conn_options =
            sea_orm::ConnectOptions::new("postgres://roadster:roadster@localhost:5432/example_dev");
        db_conn_options.connect_lazy(true);
        builder
            .db_conn_options(db_conn_options)
            .db_conn_options_provider(|config| Ok(sea_orm::ConnectOptions::from(&config.database)))
    };

    // Provide your custom state via the `state_provider` method.
    let builder = builder.state_provider(move |app_context| {
        Ok(AppState {
            app_context,
            custom_state: custom_state.clone(),
        })
    });

    // Lifecycle handlers can either be provided directly or via a provider callback. Each can be
    // called multiple times to register multiple handlers (however, registering duplicate handlers
    // will cause an error).
    let builder = builder
        .add_lifecycle_handler(ExampleLifecycleHandler::new("example1"))
        .add_lifecycle_handler_provider(|registry, _state| {
            registry.register(ExampleLifecycleHandler::new("example2"))?;
            Ok(())
        });

    // Health checks can either be provided directly or via a provider callback. Each can be called
    // multiple times to register multiple health checks (however, registering duplicate checks
    // will cause an error).
    let builder = builder
        .add_health_check(ExampleHealthCheck::new("example1"))
        .add_health_check(ExampleHealthCheck::new("example2"))
        .add_health_check_provider(|registry, _state| {
            registry.register(ExampleHealthCheck::new("example3"))?;
            Ok(())
        });

    // Services can either be provided directly or via a provider callback. Each can be called
    // multiple times to register multiple services (however, registering duplicate services
    // will cause an error).
    let builder = builder
        .add_service(
            FunctionService::builder()
                .name("example".to_string())
                .function(example_fn_service)
                .build(),
        )
        .add_service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        HttpService::builder(Some(BASE), state).api_router(http::routes(BASE)),
                    )
                    .await?;
                Ok(())
            })
        })
        .add_service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        SidekiqWorkerService::builder(state)
                            .await?
                            .register_worker(ExampleWorker::default())?,
                    )
                    .await?;
                Ok(())
            })
        });

    let builder = builder.graceful_shutdown_signal_provider(|_state| {
        Box::pin(async {
            let _output: () = future::pending().await;
        })
    });

    let app: App = builder.build();

    app.run().await
}

fn metadata() -> AppMetadata {
    AppMetadata::builder()
        .version(env!("VERGEN_GIT_SHA").to_string())
        .build()
}

async fn example_fn_service(_state: AppState, _token: CancellationToken) -> RoadsterResult<()> {
    Ok(())
}
