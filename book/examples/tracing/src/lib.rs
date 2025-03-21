use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        .tracing_initializer(|config| {
            // Implement custom logic here. See Roadster's implementation
            // for an example: https://docs.rs/roadster/latest/roadster/tracing/fn.init_tracing.html
            todo!("Custom tracing initialization logic")
        })
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        .add_service_provider(move |_registry, _state| {
            Box::pin(async move { todo!("Add services here.") })
        })
        .build()
}
