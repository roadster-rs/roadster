use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

pub fn app_with_context_extension() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(Ok)
        // Register custom data to be added to the `AppContext`.
        .context_extension_provider(|_config, registry| {
            Box::pin(async move {
                registry.register("Custom String context".to_string())?;
                Ok(())
            })
        })
        .build()
}
