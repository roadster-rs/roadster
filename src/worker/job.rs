use uuid::Uuid;

// Todo: Not sure if this should be public yet.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
pub(crate) struct Job {
    pub(crate) metadata: JobMetadata,
    pub(crate) args: serde_json::Value,
}

// Todo: Not sure if this should be public yet.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
pub(crate) struct JobMetadata {
    #[builder(default = Uuid::now_v7().to_string())]
    pub(crate) id: String,
    #[builder(into)]
    pub(crate) worker_name: String,
    pub(crate) periodic: Option<PeriodicConfig>,
}

// Todo: Not sure if this should be public yet.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
pub(crate) struct PeriodicConfig {
    pub(crate) hash: u64,
    pub(crate) schedule: cron::Schedule,
}

#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
impl From<&crate::worker::PeriodicArgsJson> for Job {
    fn from(value: &crate::worker::PeriodicArgsJson) -> Self {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let mut hash = DefaultHasher::new();
        value.hash(&mut hash);
        let hash = hash.finish();

        Job::builder()
            .args(value.args.clone())
            .metadata(
                JobMetadata::builder()
                    .worker_name(value.worker_name.clone())
                    .periodic(
                        PeriodicConfig::builder()
                            .hash(hash)
                            .schedule(value.schedule.clone())
                            .build(),
                    )
                    .build(),
            )
            .build()
    }
}

#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
pub(crate) fn periodic_hash<H: std::hash::Hasher>(
    hasher: &mut H,
    worker_name: &str,
    schedule: &cron::Schedule,
    value: &serde_json::Value,
) {
    use std::hash::Hash;

    worker_name.hash(hasher);
    schedule.to_string().hash(hasher);
    value.hash(hasher);
}

#[cfg(test)]
#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
mod tests {
    use crate::testing::snapshot::TestCase;
    use crate::worker::job::{Job, JobMetadata, PeriodicConfig};
    use cron::Schedule;
    use insta::{assert_json_snapshot, assert_snapshot};
    use rstest::{fixture, rstest};
    use std::hash::DefaultHasher;
    use std::hash::Hasher;
    use std::str::FromStr;

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case("a", Schedule::from_str("* * * * * *").unwrap(), serde_json::json!({"foo": "bar"}))]
    #[case("b", Schedule::from_str("*/10 * * * * *").unwrap(), serde_json::json!({"foo": "baz"}))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn periodic_hash(
        _case: TestCase,
        #[case] name: &str,
        #[case] schedule: Schedule,
        #[case] value: serde_json::Value,
    ) {
        let mut hasher = DefaultHasher::new();
        super::periodic_hash(&mut hasher, name, &schedule, &value);
        assert_snapshot!(hasher.finish());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn job_ser_and_deser() {
        let job = Job::builder()
            .args(serde_json::json!({"foo": "bar"}))
            .metadata(
                JobMetadata::builder()
                    .worker_name("foo")
                    .periodic(
                        PeriodicConfig::builder()
                            .hash(1234) // fake hash
                            .schedule(Schedule::from_str("* * * * * *").unwrap())
                            .build(),
                    )
                    .build(),
            )
            .build();

        let ser = serde_json::to_value(&job).unwrap();

        let job_deser: Job = serde_json::from_value(ser).unwrap();

        assert_eq!(job, job_deser);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn job_serde() {
        let _case = TestCase::new();

        let job = Job::builder()
            .args(serde_json::json!({"foo": "bar"}))
            .metadata(
                JobMetadata::builder()
                    .worker_name("foo")
                    .periodic(
                        PeriodicConfig::builder()
                            .hash(1234) // fake hash
                            .schedule(Schedule::from_str("* * * * * *").unwrap())
                            .build(),
                    )
                    .build(),
            )
            .build();

        assert_json_snapshot!(job);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn job_serde_no_periodic() {
        let _case = TestCase::new();

        let job = Job::builder()
            .args(serde_json::json!({"foo": "bar"}))
            .metadata(JobMetadata::builder().worker_name("foo").build())
            .build();

        assert_json_snapshot!(job);
    }
}
