use crate::app::context::AppContext;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::worker::{Enqueuer, Worker, enqueue};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, instrument};

pub struct PgEnqueuer;

#[async_trait]
impl Enqueuer for PgEnqueuer {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, ArgsRef, E>(state: &S, args: ArgsRef) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let id = context.pgmq().send(queue, &job).await?;
                debug!(job.msg_id = id, "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed<W, S, Args, ArgsRef, E>(
        state: &S,
        args: ArgsRef,
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let id = context
                    .pgmq()
                    .send_delay(queue, &job, delay.as_secs())
                    .await?;
                debug!(job.msg_id = id, job.delay = delay.as_secs(), "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<W, S, Args, ArgsRef, E>(
        state: &S,
        args: &[ArgsRef],
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let ids = context.pgmq().send_batch(queue, &jobs).await?;
                debug!(count = ids.len(), "Jobs enqueued");
                if debug_tracing_enabled(context.config()) {
                    ids.iter()
                        .for_each(|id| debug!(job.msg_id = id, "Job enqueued"));
                }
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed<W, S, Args, ArgsRef, E>(
        state: &S,
        args: &[ArgsRef],
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let ids = context
                    .pgmq()
                    .send_batch_delay(queue, &jobs, delay.as_secs())
                    .await?;
                debug!(count = ids.len(), delay = delay.as_secs(), "Jobs enqueued");
                if debug_tracing_enabled(context.config()) {
                    ids.iter().for_each(|id| {
                        debug!(job.msg_id = id, job.delay = delay.as_secs(), "Job enqueued")
                    });
                }
                Ok(())
            },
        )
        .await
    }
}

fn debug_tracing_enabled(config: &AppConfig) -> bool {
    tracing::Level::from_str(&config.tracing.level)
        .map(|level| level >= tracing::Level::DEBUG)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use rstest::{fixture, rstest};

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[fixture]
    fn config() -> AppConfig {
        AppConfig::test(None).unwrap()
    }

    #[rstest]
    #[case("trace", true)]
    #[case("debug", true)]
    #[case("info", false)]
    #[case("warn", false)]
    #[case("error", false)]
    #[case("invalid", false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn debug_tracing_enabled(
        _case: TestCase,
        mut config: AppConfig,
        #[case] level: &str,
        #[case] expected: bool,
    ) {
        config.tracing.level = level.to_owned();
        let enabled = super::debug_tracing_enabled(&config);
        assert_eq!(enabled, expected);
    }
}
