use crate::app::App;
use crate::app_context::AppContext;
use crate::service::worker::sidekiq::builder::SidekiqWorkerServiceBuilder;
use crate::service::AppService;
use async_trait::async_trait;
use sidekiq::Processor;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

pub struct SidekiqWorkerService {
    pub(crate) processor: Processor,
}

#[async_trait]
impl<A: App> AppService<A> for SidekiqWorkerService {
    fn name() -> String
    where
        Self: Sized,
    {
        "sidekiq".to_string()
    }

    fn enabled(context: &AppContext, _state: &A::State) -> bool
    where
        Self: Sized,
    {
        let sidekiq_config = &context.config.service.sidekiq;
        if !sidekiq_config.common.enabled(context) {
            debug!("Sidekiq is not enabled in the config.");
            return false;
        }
        if sidekiq_config.custom.num_workers == 0 {
            debug!("Sidekiq configured with 0 worker tasks.");
            return false;
        }
        if sidekiq_config.custom.queues.is_empty() {
            debug!("Sidekiq configured with 0 worker queues.");
            return false;
        }
        if context.redis_fetch.is_none() {
            debug!("No 'redis-fetch' pool connections available.");
            return false;
        }
        true
    }

    async fn run(
        &self,
        _app_context: Arc<AppContext>,
        _app_state: Arc<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        let processor = self.processor.clone();
        let sidekiq_cancel_token = processor.get_cancellation_token();

        tokio::select! {
            _ = cancel_token.cancelled() => {
                info!("The app is shutting down, stopping the sidekiq processor.")
            },
            _ = sidekiq_cancel_token.cancelled() => {
                info!("The sidekiq cancellation token was cancelled, shutting down the app.")
            },
            _ = processor.run() => {
                info!("The sidekiq processor exited, shutting down the app.")
            },
        }

        cancel_token.cancel();
        sidekiq_cancel_token.cancel();

        Ok(())
    }
}

impl SidekiqWorkerService {
    pub async fn builder<A>(
        context: Arc<AppContext>,
        state: Arc<A::State>,
    ) -> anyhow::Result<SidekiqWorkerServiceBuilder<A>>
    where
        A: App + 'static,
    {
        SidekiqWorkerServiceBuilder::with_default_processor(context, state, None).await
    }
}
