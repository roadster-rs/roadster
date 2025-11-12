#[cfg(feature = "db-sql")]
pub mod db;
pub mod default;
pub mod registry;

use crate::app::context::AppContext;
use crate::app::{App, PreparedAppWithoutCli};
use async_trait::async_trait;
use axum_core::extract::FromRef;

/// Trait used to hook into various stages of the app's lifecycle.
///
/// This trait has some overlap with the [`crate::health::check::HealthCheck`]. For example, both
/// traits have methods that are called during app startup before the app's services are started.
/// However, health checks are intended to be used both during app startup and in the
/// `/api/_health` API endpoint. This enforces a constraint on the
/// [`crate::health::check::HealthCheck`] trait that doesn't exist for the [`AppLifecycleHandler`]
/// trait -- the [`crate::health::check::HealthCheck`] is stored in the
/// [`crate::app::context::AppContext`], which means it can't take the state as a type parameter and
/// therefore also can't take it as parameter to its methods. Because this constraint only exists
/// for the health check use case, we split out the other lifecycle hooks into a separate trait that
/// allows better ergonomics -- instead of needing to store a weak reference to the app context as
/// in a health check, [`AppLifecycleHandler`] can simply accept the context as a parameter on its
/// methods.
///
/// The app's lifecycle generally looks something like this:
/// 1. Parse the [`crate::config::AppConfig`]
/// 2. Initialize tracing to enable logs/traces
/// 3. Build the [`crate::app::context::AppContext`] and the [`crate::app::App`]'s custom state
/// 4. Run the roadster/app CLI command, if one was specified when the app was started
/// 5. Register [`AppLifecycleHandler`]s, [`crate::health::check::HealthCheck`]s, and
///    [`crate::service::Service`]s
/// 6. Run the app's registered [`AppLifecycleHandler::before_health_checks`] hooks.
/// 7. Run the registered [`crate::health::check::HealthCheck`]s
/// 8. Run the app's registered [`AppLifecycleHandler::before_services`] hooks.
/// 9. Run the registered [`crate::service::Service`]s
/// 10. Wait for a shutdown signal, e.g., `Ctrl+c` or a custom signal from
///    [`crate::app::App::graceful_shutdown_signal`], and stop the [`crate::service::Service`]s
///    when the signal is received.
/// 11. Run Roadster's graceful shutdown logic
/// 12. Run the app's registered [`AppLifecycleHandler::on_shutdown`] hooks.
#[cfg_attr(test, mockall::automock(type Error = crate::error::Error;))]
#[async_trait]
pub trait AppLifecycleHandler<A, S>: Send + Sync
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: 'static + App<S>,
{
    type Error: Send + Sync + std::error::Error;

    /// The name of the [`AppLifecycleHandler`].
    fn name(&self) -> String;

    /// Whether the [`AppLifecycleHandler`] is enabled.
    fn enabled(&self, #[allow(unused_variables)] state: &S) -> bool {
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
    fn priority(&self, #[allow(unused_variables)] state: &S) -> i32 {
        0
    }

    /// This method is run right before the app's [`crate::health::check::HealthCheck`]s during
    /// app startup.
    async fn before_health_checks(
        &self,
        #[allow(unused_variables)] prepared_app: &PreparedAppWithoutCli<A, S>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// This method is run right before the app's [`crate::service::Service`]s are started.
    async fn before_services(
        &self,
        #[allow(unused_variables)] prepared_app: &PreparedAppWithoutCli<A, S>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// This method is run after the app's [`crate::service::Service`]s have stopped.
    async fn on_shutdown(&self, #[allow(unused_variables)] state: &S) -> Result<(), Self::Error> {
        Ok(())
    }
}
