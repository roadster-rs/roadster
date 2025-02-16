mod http;

use roadster::app::context::AppContext;
use roadster::app::RoadsterApp;
use roadster::service::http::service::HttpService;
use roadster::util::empty::Empty;

type App = RoadsterApp<AppContext, Empty, Empty>;

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
