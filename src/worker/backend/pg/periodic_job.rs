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

impl From<&crate::worker::PeriodicArgsJson> for PeriodicJob {
    fn from(value: &crate::worker::PeriodicArgsJson) -> Self {
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
