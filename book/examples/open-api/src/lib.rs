use crate::http::http_service;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

mod http;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry.register_builder(http_service(state)).await?;
                Ok(())
            })
        })
        .build()
}
