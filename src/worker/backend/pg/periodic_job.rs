use crate::worker::PeriodicArgsJson;
use crate::worker::job::JobMetadata;

// Todo: Not sure if this should be public yet.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct PeriodicJob {
    pub(crate) metadata: JobMetadata,
    pub(crate) periodic: PeriodicConfig,
    pub(crate) args: serde_json::Value,
}

// Todo: Not sure if this should be public yet.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct PeriodicConfig {
    pub(crate) hash: u64,
    pub(crate) schedule: cron::Schedule,
}

impl From<&PeriodicArgsJson> for PeriodicJob {
    fn from(value: &PeriodicArgsJson) -> Self {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let mut hash = DefaultHasher::new();
        value.hash(&mut hash);
        let hash = hash.finish();

        PeriodicJob::builder()
            .args(value.args.clone())
            .metadata(
                JobMetadata::builder()
                    .worker_name(value.worker_name.clone())
                    .build(),
            )
            .periodic(
                PeriodicConfig::builder()
                    .hash(hash)
                    .schedule(value.schedule.clone())
                    .build(),
            )
            .build()
    }
}

#[cfg(test)]
#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
mod tests {
    use crate::testing::snapshot::TestCase;
    use crate::worker::PeriodicArgsJson;
    use crate::worker::backend::pg::periodic_job::{PeriodicConfig, PeriodicJob};
    use crate::worker::job::JobMetadata;
    use cron::Schedule;
    use insta::{assert_json_snapshot, assert_snapshot};
    use rstest::{fixture, rstest};
    use std::hash::{DefaultHasher, Hasher};
    use std::str::FromStr;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn periodic_job_ser_and_deser() {
        let job = super::PeriodicJob::builder()
            .args(serde_json::json!({"foo": "bar"}))
            .metadata(JobMetadata::builder().worker_name("foo").build())
            .periodic(
                PeriodicConfig::builder()
                    .hash(1234) // fake hash
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            )
            .build();

        let ser = serde_json::to_value(&job).unwrap();

        let job_deser: PeriodicJob = serde_json::from_value(ser).unwrap();

        assert_eq!(job, job_deser);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn periodic_job_serde() {
        let _case = TestCase::new();

        let job = super::PeriodicJob::builder()
            .args(serde_json::json!({"foo": "bar"}))
            .metadata(JobMetadata::builder().worker_name("foo").build())
            .periodic(
                PeriodicConfig::builder()
                    .hash(1234) // fake hash
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            )
            .build();

        assert_json_snapshot!(job);
    }

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
    fn periodic_args_json_hash(periodic_args_json: PeriodicArgsJson) {
        let mut hasher = DefaultHasher::new();
        crate::worker::job::periodic_hash(
            &mut hasher,
            &periodic_args_json.worker_name,
            &periodic_args_json.schedule,
            &periodic_args_json.args,
        );
        assert_snapshot!(hasher.finish());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn job_from_periodic_args_hash(periodic_args_json: PeriodicArgsJson) {
        let job = PeriodicJob::from(&periodic_args_json);
        let mut hasher = DefaultHasher::new();
        crate::worker::job::periodic_hash(
            &mut hasher,
            &job.metadata.worker_name,
            &job.periodic.schedule,
            &job.args,
        );
        assert_eq!(hasher.finish(), job.periodic.hash);
    }
}
