use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::AppService;
use crate::worker::backend::pg::processor::Processor;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, instrument};

pub(crate) const NAME: &str = "worker-postgres";

pub(crate) fn enabled(context: &AppContext) -> bool {
    let config = &context.config().service.worker.pg;
    if !config.common.enabled(context) {
        debug!("Postgres worker service is not enabled in the config.");
        return false;
    }

    let dedicated_workers: u64 = config
        .custom
        .common
        .queue_config
        .values()
        .map(|config| config.num_workers.unwrap_or_default() as u64)
        .sum();
    if config.custom.common.num_workers == 0 && dedicated_workers == 0 {
        debug!("Postgres worker service configured with 0 worker tasks.");
        return false;
    }

    let queues_empty = if let Some(queues) = config.custom.common.queues.as_ref() {
        queues.is_empty()
    } else {
        true
    };

    if queues_empty && dedicated_workers == 0 {
        debug!("Postgres worker service configured with 0 worker queues.");
        return false;
    }

    true
}

pub struct PgWorkerService<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) processor: Processor<S>,
}

impl<S> PgWorkerService<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    // Todo: use builder pattern?
    pub(crate) fn new(processor: Processor<S>) -> Self {
        Self { processor }
    }
}

#[async_trait]
impl<A, S> AppService<A, S> for PgWorkerService<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        enabled(&AppContext::from_ref(state))
    }

    #[instrument(skip_all)]
    async fn before_run(&self, state: &S) -> RoadsterResult<()> {
        self.processor.initialize_queues().await?;
        // remove_stale_periodic_jobs(&mut conn, &context, &self.registered_periodic_workers).await
        Ok(())
    }

    async fn run(
        self: Box<Self>,
        _state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let processor = self.processor;
        let processor_cancel_token = processor.cancellation_token();

        let mut join_set = JoinSet::new();

        {
            let token = cancel_token.clone();
            join_set.spawn(Box::pin(async move {
                token.clone().cancelled().await;
            }));
        }

        {
            let token = processor_cancel_token.clone();
            join_set.spawn(Box::pin(async move {
                token.clone().cancelled().await;
            }));
        }

        join_set.spawn(processor.run());

        while let Some(result) = join_set.join_next().await {
            // Once any of the tasks finish, cancel all the cancellation tokens to ensure
            // the processor and the app shut down gracefully.
            cancel_token.cancel();
            processor_cancel_token.cancel();
            if let Err(join_err) = result {
                error!(
                    "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
                );
            }
        }

        Ok(())
    }
}
