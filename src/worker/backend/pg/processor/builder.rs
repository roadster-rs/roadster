use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::Worker;
use crate::worker::backend::pg::processor::{
    PgProcessor, PgProcessorError, ProcessorInner, WorkerWrapper,
};
use axum_core::extract::FromRef;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use tracing::{error, info};

#[non_exhaustive]
pub struct PgProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) inner: ProcessorInner<S>,
}

#[derive(bon::Builder)]
#[non_exhaustive]
pub struct PeriodicArgs<Args>
where
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    args: Args,
    schedule: Schedule,
}

#[derive(Clone, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct PeriodicArgsJson {
    pub(crate) args: serde_json::Value,
    pub(crate) worker_name: String,
    pub(crate) schedule: Schedule,
}

impl Ord for PeriodicArgsJson {
    fn cmp(&self, other: &Self) -> Ordering {
        self.worker_name
            .cmp(&other.worker_name)
            .then(self.schedule.to_string().cmp(&other.schedule.to_string()))
            .then(
                serde_json::to_string(&self.args)
                    .unwrap_or_default()
                    .cmp(&serde_json::to_string(&other.args).unwrap_or_default()),
            )
    }
}

impl PartialOrd for PeriodicArgsJson {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> PgProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            inner: ProcessorInner {
                state: state.clone(),
                queues: Default::default(),
                workers: Default::default(),
                periodic_workers: Default::default(),
            },
        }
    }

    pub fn build(self) -> PgProcessor<S> {
        PgProcessor::new(self.inner)
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering PG worker");

        self.register_internal(worker, name, false)?;

        Ok(self)
    }

    pub fn register_periodic<W, Args, E>(
        mut self,
        worker: W,
        periodic_args: PeriodicArgs<Args>,
    ) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering periodic PG worker");

        self.register_internal(worker, name.clone(), true)?;

        let periodic_args = PeriodicArgsJson::builder()
            .args(serde_json::to_value(periodic_args.args)?)
            .worker_name(name.clone())
            .schedule(periodic_args.schedule)
            .build();

        if let Some(replaced) = self.inner.periodic_workers.replace(periodic_args) {
            return Err(PgProcessorError::AlreadyRegisteredPeriodic(
                replaced.worker_name,
                replaced.schedule,
                replaced.args,
            )
            .into());
        }

        Ok(self)
    }

    fn register_internal<W, Args, E>(
        &mut self,
        worker: W,
        name: String,
        skip_duplicate: bool,
    ) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let context = AppContext::from_ref(&self.inner.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.inner.state);

        let queue = worker_enqueue_config
            .queue
            .as_ref()
            .or(enqueue_config.queue.as_ref());
        let queue = if let Some(queue) = queue {
            queue
        } else {
            error!(
                worker_name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(PgProcessorError::NoQueue(W::name()).into());
        };
        self.inner.queues.insert(queue.to_owned());

        if self
            .inner
            .workers
            .insert(
                name.clone(),
                WorkerWrapper::new(&self.inner.state, worker, worker_enqueue_config)?,
            )
            .is_some()
            && !skip_duplicate
        {
            return Err(PgProcessorError::AlreadyRegistered(name).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod periodic_args {
        use crate::worker::backend::pg::processor::builder::PeriodicArgsJson;
        use crate::worker::job::Job;
        use cron::Schedule;
        use insta::assert_json_snapshot;
        use rstest::{fixture, rstest};
        use std::str::FromStr;

        #[fixture]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn periodic_args_json() -> PeriodicArgsJson {
            PeriodicArgsJson::builder()
                .worker_name("a".to_string())
                .schedule(Schedule::from_str("* * * * * *").unwrap())
                .args(serde_json::json!({"foo": "bar"}))
                .build()
        }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn periodic_args_json_ord_name(periodic_args_json: PeriodicArgsJson) {
            let mut b = periodic_args_json.clone();
            b.worker_name = "b".to_string();
            assert!(periodic_args_json < b);
        }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn periodic_args_json_ord_schedule(periodic_args_json: PeriodicArgsJson) {
            let mut b = periodic_args_json.clone();
            b.schedule = Schedule::from_str("*/10 * * * * *").unwrap();
            assert!(periodic_args_json < b);
        }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn periodic_args_json_ord_args(periodic_args_json: PeriodicArgsJson) {
            let mut b = periodic_args_json.clone();
            b.args = serde_json::json!({"foo": "baz"});
            assert!(periodic_args_json < b);
        }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn job_from_periodic_args(periodic_args_json: PeriodicArgsJson) {
            let job = Job::from(&periodic_args_json);
            assert_json_snapshot!(job);
        }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn job_from_periodic_args_hash(periodic_args_json: PeriodicArgsJson) {
            let job = Job::from(&periodic_args_json);
            let hash = crate::worker::job::periodic_hash(
                &job.metadata.worker_name,
                &job.metadata.periodic.as_ref().unwrap().schedule,
                &job.args,
            );
            assert_eq!(hash, job.metadata.periodic.unwrap().hash);
        }
    }
}
