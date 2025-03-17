use crate::worker::email::smtp::EmailConfirmationPlainText;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;

pub mod model;
pub mod worker;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry
                    .register_builder(
                        SidekiqWorkerService::builder(state)
                            .await?
                            .register_worker(EmailConfirmationPlainText::new(state))?,
                    )
                    .await?;
                Ok(())
            })
        })
        .build()
}
