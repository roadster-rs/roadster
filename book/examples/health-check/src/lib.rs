mod example_check;

use crate::example_check::ExampleCheck;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        // Register custom health check(s)
        .add_health_check_provider(|registry, state| {
            registry.register(ExampleCheck::new(state))?;
            Ok(())
        })
        .add_service_provider(move |_registry, _state| {
            Box::pin(async move { todo!("Add services here.") })
        })
        .build()
}
