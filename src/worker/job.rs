use crate::worker::backend::pg::processor::builder::PeriodicArgsJson;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::hash::{DefaultHasher, Hash, Hasher};

// Todo: Not sure if this should be public yet.
#[derive(Serialize, Deserialize, bon::Builder)]
pub(crate) struct Job {
    pub(crate) metadata: JobMetadata,
    pub(crate) args: serde_json::Value,
}

// Todo: Not sure if this should be public yet.
#[skip_serializing_none]
#[derive(Serialize, Deserialize, bon::Builder)]
pub(crate) struct JobMetadata {
    pub(crate) worker_name: String,
    pub(crate) periodic: Option<PeriodicConfig>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, bon::Builder)]
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
