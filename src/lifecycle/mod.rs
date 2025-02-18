#[cfg(feature = "db-sql")]
pub mod db;
pub mod default;
pub mod registry;

use crate::app::context::AppContext;
use crate::app::{App, PreRunAppState};
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;

/// Trait used to hook into various stages of the app's lifecycle.
///
/// The app's lifecycle generally looks something like this:
/// 1. Parse the [`crate::config::AppConfig`]
/// 2. Initialize tracing to enable logs/traces
/// 3. Build the [`crate::app::context::AppContext`] and the [`crate::app::App`]'s custom state
/// 4. Run the roadster/app CLI command, if one was specified when the app was started
/// 5. Register [`AppLifecycleHandler`]s, [`crate::health::check::HealthCheck`]s, and
///    [`crate::service::AppService`]s
/// 6. Run the app's registered [`AppLifecycleHandler::before_health_checks`] hooks.
/// 7. Run the registered [`crate::health::check::HealthCheck`]s
/// 8. Run the app's registered [`AppLifecycleHandler::before_services`] hooks.
/// 9. Run the registered [`crate::service::AppService`]s
/// 10. Wait for a shutdown signal, e.g., `Ctrl+c` or a custom signal from
///    [`crate::app::App::graceful_shutdown_signal`], and stop the [`crate::service::AppService`]s
///    when the signal is received.
/// 11. Run Roadster's graceful shutdown logic
/// 12. Run the app's registered [`AppLifecycleHandler::on_shutdown`] hooks.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AppLifecycleHandler<A, S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    /// The name of the [`AppLifecycleHandler`].
    fn name(&self) -> String;

    /// Whether the [`AppLifecycleHandler`] is enabled.
    fn enabled(&self, _state: &S) -> bool {
        true
    }

    /// Used to determine the order in which the [`AppLifecycleHandler`] will run when during app
    /// startup. Smaller numbers will run before larger numbers. For example, a
    /// [`AppLifecycleHandler`] with priority `-10` will run before a [`AppLifecycleHandler`]
    /// with priority `10`.
    ///
    /// If two [`AppLifecycleHandler`]s have the same priority, they are not guaranteed to run
    /// in any particular order relative to each other. This may be fine for many
    /// [`AppLifecycleHandler`]s .
    ///
    /// If the order in which your [`AppLifecycleHandler`] runs doesn't particularly matter, it's
    /// generally safe to set its priority as `0`.
    fn priority(&self, _state: &S) -> i32 {
        0
    }

    /// This method is run right before the app's [`crate::health::check::HealthCheck`]s during
    /// app startup.
    async fn before_health_checks(
        &self,
        _prepared_app: &PreRunAppState<A, S>,
    ) -> RoadsterResult<()> {
        Ok(())
    }

    /// This method is run right before the app's [`crate::service::AppService`]s are started.
    async fn before_services(&self, _prepared_app: &PreRunAppState<A, S>) -> RoadsterResult<()> {
        Ok(())
    }

    /// This method is run after the app's [`crate::service::AppService`]s have stopped.
    async fn on_shutdown(&self, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }
}
