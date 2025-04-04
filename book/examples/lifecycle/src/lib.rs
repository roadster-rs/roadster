mod example_lifecycle_handler;

use crate::example_lifecycle_handler::ExampleLifecycleHandler;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        // Register custom lifecycle handler(s)
        .add_lifecycle_handler_provider(|registry, _state| {
            registry.register(ExampleLifecycleHandler)?;
            Ok(())
        })
        .add_service_provider(move |_registry, _state| {
            Box::pin(async move { todo!("Add services here.") })
        })
        .build()
}
