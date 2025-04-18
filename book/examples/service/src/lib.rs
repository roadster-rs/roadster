mod function;
#[cfg(feature = "grpc")]
mod grpc;
mod http;
mod worker;

use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::http::service::HttpService;

type App = RoadsterApp<AppContext>;

fn build_app() -> App {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry
                    .register_builder(HttpService::builder(Some("/api"), state))
                    .await?;
                Ok(())
            })
        })
        .build()
}
