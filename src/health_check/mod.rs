#[cfg(feature = "db-sql")]
pub mod database;
pub mod default;
#[cfg(feature = "sidekiq")]
pub mod sidekiq;

use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use async_trait::async_trait;

/// Trait used to check the health of the app before its services start up.
///
/// This is a separate trait, vs adding a "health check" method to `AppService`, to allow defining
/// health checks that apply to multiple services. For example, most services would require
/// the DB and Redis connections to be valid, so we would want to perform a check for these
/// resources a single time before starting any service instead of once for every service that
/// needs the resources.
///
/// Another benefit of using a separate trait is, because the health checks are decoupled from
/// services, they can potentially be used in other parts of the app. For example, they could
/// be used to implement a "health check" API endpoint.
// Todo: Use the `HealthCheck` trait to implement the "health check" api - https://github.com/roadster-rs/roadster/issues/241
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait HealthCheck<A: App + 'static>: Send + Sync {
    /// The name of the health check.
    fn name(&self) -> String;

    /// Whether the health check is enabled. If the health check is not enabled, Roadster will not
    /// run it. However, if a consumer wants, they can certainly create a [HealthCheck] instance
    /// and directly call `HealthCheck#check` even if `HealthCheck#enabled` returns `false`.
    fn enabled(&self, context: &AppContext<A::State>) -> bool;

    /// Run the health check.
    async fn check(&self, app_context: &AppContext<A::State>) -> RoadsterResult<()>;
}
