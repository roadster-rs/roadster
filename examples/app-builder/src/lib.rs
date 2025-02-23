use crate::api::http;
use crate::app_state::AppState;
use crate::config::example_async_source::ExampleAsyncSource;
use crate::config::example_async_source_with_env::ExampleAsyncSourceWithEnv;
use crate::health::check::example::ExampleHealthCheck;
use crate::lifecycle::example::ExampleLifecycleHandler;
use crate::worker::example::ExampleWorker;
use cfg_if::cfg_if;
use roadster::app::metadata::AppMetadata;
use roadster::app::{RoadsterApp, RoadsterAppBuilder};
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
use roadster::service::http::service::HttpService;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;
use std::future;
use tokio_util::sync::CancellationToken;

pub mod api;
pub mod app_state;
pub mod config;
pub mod health;
pub mod lifecycle;
#[cfg(feature = "db-sea-orm")]
pub mod model;
pub mod worker;

cfg_if! {
if #[cfg(feature = "cli")] {
    pub type App = RoadsterApp<AppState, api::cli::AppCli>;
} else {
    pub type App = RoadsterApp<AppState>;
}
}

const BASE: &str = "/api";

pub fn build_app() -> App {
    let custom_state = "custom".to_string();

    let builder: RoadsterAppBuilder<AppState, _> = RoadsterApp::builder()
        .tracing_initializer(|config| roadster::tracing::init_tracing(config, &metadata()));

    // If your application needs to load configuration fields (particularly sensitive ones) from an
    // external service, such as AWS or GCS secrets manager services, you can load them via
    // an `AsyncSource`.
    let builder = builder
        .add_async_config_source(ExampleAsyncSource)
        // If the `AsyncSource` needs to know which environment it's running in, e.g. in order
        // to use a different secrets manager endpoint per-environment, you can use the
        // `async_config_source_provider` hook to build the source.
        .add_async_config_source_provider(|environment| {
            Ok(Box::new(ExampleAsyncSourceWithEnv::new(environment)))
        });

    // Metadata can either be provided directly or via a provider callback. Note that the two
    // approaches are mutually exclusive, with the `metadata` method taking priority.
    let builder = builder
        .metadata(metadata())
        .metadata_provider(move |_config| Ok(metadata()));

    // Db connection options can either be provided directly with `db_conn_options` or via the
    // `sea_orm_conn_options_provider` callback. Note that the two approaches are mutually
    // exclusive, with the `db_conn_options` method taking priority.
    #[cfg(feature = "db-sea-orm")]
    let builder = {
        let mut db_conn_options =
            sea_orm::ConnectOptions::new("postgres://roadster:roadster@localhost:5432/example_dev");
        db_conn_options.connect_lazy(true);
        builder
            .sea_orm_conn_options(db_conn_options)
            .sea_orm_conn_options_provider(|config| {
                Ok(sea_orm::ConnectOptions::from(&config.database))
            })
    };

    // Roadster can automatically run the app's DB migrations on start up. Simply provide
    // the app's migrator instance (something that implements sea-orm's `MigratorTrait`).
    #[cfg(feature = "db-sea-orm")]
    let builder = { builder.sea_orm_migrator(migration::Migrator) };

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
                            .register_worker(ExampleWorker)?,
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

    builder.build()
}

fn metadata() -> AppMetadata {
    AppMetadata::builder()
        .version(env!("VERGEN_GIT_SHA").to_string())
        .build()
}

async fn example_fn_service(_state: AppState, _token: CancellationToken) -> RoadsterResult<()> {
    Ok(())
}
