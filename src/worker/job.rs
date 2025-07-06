use crate::worker::PeriodicArgsJson;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::hash::{DefaultHasher, Hash, Hasher};

// Todo: Not sure if this should be public yet.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, bon::Builder, Eq, PartialEq)]
pub(crate) struct Job {
    pub(crate) metadata: JobMetadata,
    pub(crate) args: serde_json::Value,
}

// Todo: Not sure if this should be public yet.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, bon::Builder, Eq, PartialEq)]
pub(crate) struct JobMetadata {
    #[builder(into)]
    pub(crate) worker_name: String,
    pub(crate) periodic: Option<PeriodicConfig>,
}

// Todo: Not sure if this should be public yet.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, bon::Builder, Eq, PartialEq)]
pub(crate) struct PeriodicConfig {
    pub(crate) hash: u64,
    pub(crate) schedule: Schedule,
}

impl From<&PeriodicArgsJson> for Job {
    fn from(value: &PeriodicArgsJson) -> Self {
        let hash = periodic_hash(&value.worker_name, &value.schedule, &value.args);
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

pub(crate) fn periodic_hash(
    worker_name: &str,
    schedule: &Schedule,
    value: &serde_json::Value,
) -> u64 {
    let mut hash = DefaultHasher::new();
    worker_name.hash(&mut hash);
    schedule.to_string().hash(&mut hash);
    value.hash(&mut hash);
    hash.finish()
}

#[cfg(test)]
mod tests {
    use crate::testing::snapshot::TestCase;
    use crate::worker::job::{Job, JobMetadata, PeriodicConfig};
    use cron::Schedule;
    use insta::{assert_json_snapshot, assert_snapshot};
    use rstest::{fixture, rstest};
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
        assert_snapshot!(super::periodic_hash(name, &schedule, &value));
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
}
