use roadster::app::context::AppContext;
use roadster::app::{prepare, PrepareOptions, RoadsterApp, RoadsterAppBuilder};
use roadster::error::RoadsterResult;
use roadster::service::http::service::{HttpService, OpenApiArgs};
use roadster::util::empty::Empty;

type App = RoadsterApp<AppContext, Empty, Empty>;

async fn open_api() -> RoadsterResult<()> {
    // Build the app
    let app: App = RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry
                    .register_builder(crate::http::http_service(state).await)
                    .await?;
                Ok(())
            })
        })
        .build();

    // Prepare the app
    let prepared = prepare(app, PrepareOptions::builder().build()).await?;

    // Get the `HttpService`
    let http_service = prepared.service_registry.get::<HttpService>()?;

    // Get the OpenAPI schema
    let schema = http_service.open_api_schema(&OpenApiArgs::builder().build())?;

    println!("{schema}");

    Ok(())
}
