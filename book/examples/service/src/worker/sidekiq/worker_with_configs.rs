use crate::worker::sidekiq::ExampleWorker;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::worker::backend::sidekiq::SidekiqWorkerService;
use roadster::service::worker::sidekiq::app_worker::AppWorkerConfig;
use roadster::service::worker::sidekiq::builder::SidekiqWorkerServiceBuilder;
use std::time::Duration;

async fn register_worker(
    service: SidekiqWorkerServiceBuilder<AppContext>,
    context: &AppContext,
) -> RoadsterResult<()> {
    service.register_worker_with_config(
        ExampleWorker::new(context),
        AppWorkerConfig::builder()
            .max_retries(3)
            .timeout(true)
            .max_duration(Duration::from_secs(30))
            .build(),
    )?;
    Ok(())
}
