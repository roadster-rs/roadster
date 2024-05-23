use crate::app::App;
#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::worker::sidekiq::builder::SidekiqWorkerServiceBuilder;
use crate::service::AppService;
use async_trait::async_trait;
use sidekiq::Processor;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

pub struct SidekiqWorkerService {
    pub(crate) processor: Processor,
}

#[async_trait]
impl<A: App + 'static> AppService<A> for SidekiqWorkerService {
    fn name() -> String
    where
        Self: Sized,
    {
        "sidekiq".to_string()
    }

    fn enabled(context: &AppContext<A::State>) -> bool
    where
        Self: Sized,
    {
        let sidekiq_config = &context.config().service.sidekiq;
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
        if context.redis_fetch().is_none() {
            debug!("No 'redis-fetch' pool connections available.");
            return false;
        }
        true
    }

    async fn run(
        &self,
        _app_context: &AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        let processor = self.processor.clone();
        let sidekiq_cancel_token = processor.get_cancellation_token();

        let mut join_set = JoinSet::new();
        let token = cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            token.cancelled().await;
        }));
        let token = sidekiq_cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            token.cancelled().await;
        }));
        join_set.spawn(processor.run());

        while let Some(result) = join_set.join_next().await {
            // Once any of the tasks finishes, cancel the cancellation tokens to ensure
            // the processor and the app shut down gracefully.
            cancel_token.cancel();
            sidekiq_cancel_token.cancel();
            if let Err(join_err) = result {
                error!("An error occurred when trying to join on one of the app's tasks. Error: {join_err}");
            }
        }

        Ok(())
    }
}

impl SidekiqWorkerService {
    pub async fn builder<A>(
        context: &AppContext<A::State>,
    ) -> anyhow::Result<SidekiqWorkerServiceBuilder<A>>
    where
        A: App + 'static,
    {
        SidekiqWorkerServiceBuilder::with_default_processor(context, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use bb8::Pool;
    use rstest::rstest;
    use sidekiq::RedisConnectionManager;

    #[rstest]
    #[case(false, None, 0, Default::default(), false, false)]
    #[case(true, None, 1, vec!["foo".to_string()], true, true)]
    #[case(false, Some(true), 1, vec!["foo".to_string()], true, true)]
    #[case(false, Some(false), 1, vec!["foo".to_string()], true, false)]
    #[case(true, None, 0, vec!["foo".to_string()], true, false)]
    #[case(true, None, 1, Default::default(), true, false)]
    #[case(true, None, 1, vec!["foo".to_string()], false, false)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn enabled(
        #[case] default_enabled: bool,
        #[case] sidekiq_enabled: Option<bool>,
        #[case] num_workers: u32,
        #[case] queues: Vec<String>,
        #[case] has_redis_fetch: bool,
        #[case] expected_enabled: bool,
    ) {
        let mut config = AppConfig::empty(None).unwrap();
        config.service.default_enable = default_enabled;
        config.service.sidekiq.common.enable = sidekiq_enabled;
        config.service.sidekiq.custom.num_workers = num_workers;
        config.service.sidekiq.custom.queues = queues;

        let mut context = MockAppContext::default();
        context.expect_config().return_const(config);

        let pool = if has_redis_fetch {
            let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
            let pool = Pool::builder().build_unchecked(redis_fetch);
            Some(pool)
        } else {
            None
        };
        context.expect_redis_fetch().return_const(pool);

        assert_eq!(
            <SidekiqWorkerService as AppService<MockApp>>::enabled(&context),
            expected_enabled
        );
    }
}
