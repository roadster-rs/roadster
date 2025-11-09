use roadster::app::context::AppContext;
use roadster::app::{PrepareOptions, RoadsterApp, prepare};
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster::service::http::service::OpenApiArgs;

type App = RoadsterApp<AppContext>;

async fn open_api() -> RoadsterResult<()> {
    // Build the app
    let app: App = RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry
                    .register_builder(crate::http::http_service(state))
                    .await?;
                Ok(())
            })
        })
        .build();

    // Prepare the app
    let prepared = prepare(app, PrepareOptions::builder().build()).await?;

    // Get the OpenAPI schema from the `HttpService`
    let schema = prepared
        .service_registry
        .invoke(async |srvc: &HttpService| srvc.open_api_schema(&OpenApiArgs::builder().build()))
        .await??;

    println!("{schema}");

    Ok(())
}
