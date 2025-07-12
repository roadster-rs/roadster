use crate::worker::email::smtp::EmailConfirmationPlainText;
use leptos::prelude::Render;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::worker::PgWorkerService;
use roadster::worker::backend::pg::processor::PgProcessor;

pub mod model;
pub mod worker;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                let processor = PgProcessor::builder(state)
                    .register(EmailConfirmationPlainText)?
                    .build()
                    .await?;
                registry
                    .register_service(PgWorkerService::builder().processor(processor).build())?
                    .build();
                Ok(())
            })
        })
        .build()
}
