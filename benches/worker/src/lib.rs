use crate::latch::Countdown;
use crate::worker::example::{PgExampleWorker, SidekiqExampleWorker};
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::worker::{PgWorkerService, SidekiqWorkerService};
use roadster::worker::SidekiqProcessor;
use roadster::worker::backend::pg::processor::PgProcessor;
use tokio_util::sync::CancellationToken;

pub mod latch;
pub mod worker;

pub type App = RoadsterApp<AppContext>;

pub fn build_app(latch: Countdown, cancellation_token: CancellationToken) -> App {
    let builder = RoadsterApp::builder();

    let builder = builder.state_provider(Ok);

    let builder = builder.add_service_provider(move |registry, state| {
        let latch = latch.clone();
        Box::pin(async move {
            let processor = PgProcessor::builder(state)
                .register(PgExampleWorker::builder().latch(latch.clone()).build())?
                .build()
                .await?;
            registry.register_service(PgWorkerService::builder().processor(processor).build())?;

            let processor = SidekiqProcessor::builder(state)
                .register(SidekiqExampleWorker::builder().latch(latch).build())?
                .build()
                .await?;
            registry
                .register_service(SidekiqWorkerService::builder().processor(processor).build())?;

            Ok(())
        })
    });

    let builder = builder.graceful_shutdown_signal_provider(move |_| {
        let cancellation_token = cancellation_token.clone();
        Box::pin(async move { cancellation_token.cancelled().await })
    });

    builder.build()
}
