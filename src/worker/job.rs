use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use typed_builder::TypedBuilder;

#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct Job<'a> {
    #[serde(borrow)]
    metadata: JobMetadata<'a>,
    // Using [`Cow`] instead of `&str` because `&str` will fail to deserialize if the string
    // contains escape characters. See: https://github.com/serde-rs/serde/issues/1413
    #[serde(borrow)]
    args: Cow<'a, str>,
}

#[derive(Serialize, Deserialize, TypedBuilder)]
pub struct JobMetadata<'a> {
    worker_name: &'a str,
}
