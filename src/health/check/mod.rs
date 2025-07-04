use std::time::Duration;

pub mod db;
pub mod default;
#[cfg(feature = "email")]
pub mod email;
pub mod registry;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq_enqueue;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq_fetch;

use crate::error::RoadsterResult;
use async_trait::async_trait;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{serde_as, skip_serializing_none};
use tracing::error;
use typed_builder::TypedBuilder;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CheckResponse {
    pub status: Status,
    /// Total latency of checking the health of the resource in milliseconds.
    #[builder(setter(transform = |duration: std::time::Duration| duration.as_millis()))]
    pub latency: u128,
    /// Custom health data, for example, separate latency measurements for acquiring a connection
    /// from a resource pool vs making a request with the connection.
    #[builder(
        default,
        setter(transform = |custom: impl serde::Serialize| serialize_custom(custom))
    )]
    pub custom: Option<Value>,
}

fn serialize_custom(custom: impl serde::Serialize) -> Option<Value> {
    Some(
        serde_json::to_value(custom)
            .unwrap_or_else(|err| Value::String(format!("Unable to serialize custom data: {err}"))),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Status {
    Ok,
    Err(ErrorData),
}

#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ErrorData {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(default, setter(strip_option))]
    pub msg: Option<String>,
}

/// Trait used to check the health of the app before its services start up.
///
/// This is a separate trait, vs adding a "health check" method to [`crate::service::AppService`],
/// to allow defining health checks that apply to multiple services. For example, most services
/// would require the DB and Redis connections to be valid, so we would want to perform a check for
/// these resources a single time before starting any service instead of once for every service that
/// needs the resources.
///
/// Another benefit of using a separate trait is, because the health checks are decoupled from
/// services, they can potentially be used in other parts of the app. For example, they can
/// be used to implement the "health check" API endpoint.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// The name of the [`HealthCheck`].
    fn name(&self) -> String;

    /// Whether the health check is enabled. If the health check is not enabled, Roadster will not
    /// run it. However, if a consumer wants, they can certainly create a [`HealthCheck`] instance
    /// and directly call [`HealthCheck::check`] even if [`HealthCheck::enabled`] returns `false`.
    fn enabled(&self) -> bool;

    /// Run the [`HealthCheck`].
    // Note: This is not able to take a state/AppContext type parameter because that makes it
    // not "dyn-compatible", which means it can't be made into an object. If a `HealthCheck` impl
    // needs the state/AppContext, it needs to have it as a field in its struct, and it should
    // use an `AppContextWeak` to avoid a reference cycle.
    async fn check(&self) -> RoadsterResult<CheckResponse>;
}

// This method is not used in all feature configurations.
#[allow(dead_code)]
fn missing_context_response() -> CheckResponse {
    let msg = "AppContext missing; is the app shutting down?".to_string();
    error!(msg);
    CheckResponse::builder()
        .status(Status::Err(ErrorData::builder().msg(msg).build()))
        .latency(Duration::from_secs(0))
        .build()
}

#[cfg(test)]
mod tests {
    use insta::assert_json_snapshot;

    #[test]
    fn missing_context_response() {
        assert_json_snapshot!(super::missing_context_response());
    }
}
