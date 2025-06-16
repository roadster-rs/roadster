use crate::worker::Worker;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use typed_builder::TypedBuilder;

// Todo: Not sure if this should be public yet.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub(crate) struct Job<'a> {
    #[serde(borrow)]
    metadata: JobMetadata<'a>,
    // Using [`Cow`] instead of `&str` because `&str` will fail to deserialize if the string
    // contains escape characters. See: https://github.com/serde-rs/serde/issues/1413
    #[serde(borrow)]
    args: Cow<'a, str>,
}

// Todo: Not sure if this should be public yet.
#[derive(Serialize, Deserialize, TypedBuilder)]
pub(crate) struct JobMetadata<'a> {
    worker_name: &'a str,
}
