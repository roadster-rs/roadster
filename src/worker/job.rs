use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typed_builder::TypedBuilder;

// Todo: Not sure if this should be public yet.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub(crate) struct Job {
    pub(crate) metadata: JobMetadata,
    pub(crate) args: serde_json::Value,
}

// Todo: Not sure if this should be public yet.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub(crate) struct JobMetadata {
    pub(crate) worker_name: String,
}
