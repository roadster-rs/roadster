use crate::app::context::AppContext;
use crate::app::App;
use crate::config::service::worker::sidekiq::StaleCleanUpBehavior;
use crate::error::RoadsterResult;
use crate::service::worker::sidekiq::builder::{SidekiqWorkerServiceBuilder, PERIODIC_KEY};
use crate::service::AppService;
use crate::util::redis::RedisCommands;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use sidekiq::Processor;
use std::collections::HashSet;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

pub(crate) const NAME: &str = "sidekiq";

pub(crate) fn enabled(context: &AppContext) -> bool {
    let sidekiq_config = &context.config().service.sidekiq;
    if !sidekiq_config.common.enabled(context) {
        debug!("Sidekiq is not enabled in the config.");
        return false;
    }

    let dedicated_workers: u64 = context
        .config()
        .service
        .sidekiq
        .custom
        .queue_config
        .values()
        .map(|config| config.num_workers.unwrap_or_default() as u64)
        .sum();
    if sidekiq_config.custom.num_workers == 0 && dedicated_workers == 0 {
        debug!("Sidekiq configured with 0 worker tasks.");
        return false;
    }

    if sidekiq_config.custom.queues.is_empty() && dedicated_workers == 0 {
        debug!("Sidekiq configured with 0 worker queues.");
        return false;
    }

    if context.redis_fetch().is_none() {
        debug!("No 'redis-fetch' pool connections available.");
        return false;
    }
    true
}

pub struct SidekiqWorkerService {
    pub(crate) registered_periodic_workers: HashSet<String>,
    pub(crate) processor: Processor,
}

#[async_trait]
impl<A, S> AppService<A, S> for SidekiqWorkerService
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
        let context = AppContext::from_ref(state);
        let mut conn = context.redis_enqueue().get().await?;
        remove_stale_periodic_jobs(&mut conn, &context, &self.registered_periodic_workers).await
    }

    async fn run(
        self: Box<Self>,
        _state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let processor = self.processor;
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
    pub async fn builder<S>(state: &S) -> RoadsterResult<SidekiqWorkerServiceBuilder<S>>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        SidekiqWorkerServiceBuilder::with_default_processor(state, None).await
    }
}

/// Compares the list of periodic jobs that were registered by the app during app startup with
/// the list of periodic jobs in Redis, and removes any that exist in Redis but weren't
/// registered during start up.
///
/// The jobs are only removed if the [worker.sidekiq.periodic.stale-cleanup][crate::config::worker::Periodic]
/// config is set to [auto-clean-stale][StaleCleanUpBehavior::AutoCleanStale].
///
/// This is run after all the app's periodic jobs have been registered.
async fn remove_stale_periodic_jobs<C: RedisCommands>(
    conn: &mut C,
    context: &AppContext,
    registered_periodic_workers: &HashSet<String>,
) -> RoadsterResult<()> {
    let stale_jobs = conn
        .zrange(PERIODIC_KEY.to_string(), 0, -1)
        .await?
        .into_iter()
        .filter(|job| !registered_periodic_workers.contains(job))
        .collect_vec();

    if stale_jobs.is_empty() {
        info!("No stale periodic jobs found");
        return Ok(());
    }

    if context
        .config()
        .service
        .sidekiq
        .custom
        .periodic
        .stale_cleanup
        == StaleCleanUpBehavior::AutoCleanStale
    {
        info!(
            "Removing {} stale periodic jobs:\n{}",
            stale_jobs.len(),
            stale_jobs.join("\n")
        );
        conn.zrem(PERIODIC_KEY.to_string(), stale_jobs.clone())
            .await?;
    } else {
        warn!(
            "Found {} stale periodic jobs:\n{}",
            stale_jobs.len(),
            stale_jobs.join("\n")
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use crate::util::redis::MockRedisCommands;
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
        let mut config = AppConfig::test(None).unwrap();
        config.service.default_enable = default_enabled;
        config.service.sidekiq.common.enable = sidekiq_enabled;
        config.service.sidekiq.custom.num_workers = num_workers;
        config.service.sidekiq.custom.queues = queues;

        let pool = if has_redis_fetch {
            let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
            let pool = Pool::builder().build_unchecked(redis_fetch);
            Some(pool)
        } else {
            None
        };

        let context = AppContext::test(Some(config), None, pool).unwrap();

        assert_eq!(super::enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(false, Default::default(), Default::default(), Default::default())]
    #[case(true, Default::default(), Default::default(), Default::default())]
    #[case(true, Default::default(), vec!["foo".to_string()], vec!["foo".to_string()])]
    #[case(true, vec!["foo".to_string()], vec!["foo".to_string()], Default::default())]
    #[case(true, vec!["foo".to_string()], vec!["bar".to_string()], vec!["bar".to_string()])]
    #[case(false, Default::default(), vec!["foo".to_string()], Default::default())]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn remove_stale_periodic_jobs(
        #[case] clean_stale: bool,
        #[case] registered_jobs: Vec<String>,
        #[case] jobs_in_redis: Vec<String>,
        #[case] expected_jobs_removed: Vec<String>,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        if clean_stale {
            config.service.sidekiq.custom.periodic.stale_cleanup =
                StaleCleanUpBehavior::AutoCleanStale;
        } else {
            config.service.sidekiq.custom.periodic.stale_cleanup = StaleCleanUpBehavior::Manual;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let mut redis = MockRedisCommands::default();
        redis
            .expect_zrange()
            .times(1)
            .return_once(move |_, _, _| Ok(jobs_in_redis));

        let zrem = redis.expect_zrem();
        if clean_stale && !expected_jobs_removed.is_empty() {
            zrem.times(1);
        } else {
            zrem.never();
        }
        zrem.withf(move |key, jobs| PERIODIC_KEY == key && expected_jobs_removed.iter().eq(jobs))
            .return_once(|_, _: Vec<String>| Ok(1));

        let registered_jobs: HashSet<String> =
            registered_jobs.iter().map(|s| s.to_string()).collect();

        super::remove_stale_periodic_jobs(&mut redis, &context, &registered_jobs)
            .await
            .unwrap();
    }
}
