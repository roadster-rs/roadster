use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::function::service::FunctionService;
use tokio_util::sync::CancellationToken;
use tracing::info;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        // Register the example function-based service
        .add_service(
            FunctionService::builder()
                .name("example-service")
                .function(example_service)
                .build(),
        )
        .build()
}

async fn example_service(
    _state: AppContext,
    _cancel_token: CancellationToken,
) -> RoadsterResult<()> {
    info!("Running example function-based service");
    Ok(())
}
