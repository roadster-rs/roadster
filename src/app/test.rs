use crate::app;
use crate::app::context::AppContext;
use crate::app::prepare::{PrepareOptions, PreparedAppWithoutCli};
use crate::app::{App, prepare, run};
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::service::registry::ServiceRegistry;
use axum_core::extract::FromRef;
use futures::FutureExt;
use std::convert::Infallible;
use std::panic::{AssertUnwindSafe, resume_unwind};

#[non_exhaustive]
pub struct TestAppState<A, S>
where
    A: 'static + App<S>,
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub app: A,
    pub state: S,
    pub service_registry: ServiceRegistry<S>,
}

/// Similar to [`run`], except intended to be used in tests. Does all of the same setup and
/// teardown logic as [`run`], but does not actually run the registered
/// [`crate::service::Service`]s.
///
/// Note: If the test panics, the teardown logic will only be run if the `testing.catch-panic`
/// config is set to `true`.
pub async fn run_test<A, S>(
    app: A,
    options: PrepareOptions,
    // todo: RustRover doesn't seem to recognize `AsyncFnOnce`. Does it just need an update?
    test_fn: impl std::ops::AsyncFnOnce(&TestAppState<A, S>),
) -> RoadsterResult<()>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: 'static + Send + Sync + App<S>,
{
    let result = run_test_with_result(app, options, async move |app| -> Result<(), Infallible> {
        test_fn(app).await;
        Ok(())
    })
    .await;

    if let Err((Some(err), _)) = result {
        return Err(err);
    }

    Ok(())
}

/// Similar to [`run_test`], except allows returning a [`Result`] to communicate test
/// success/failure. If the test returns an [`Err`], the teardown logic will still be run. If the
/// test returns an [`Err`], it will then be returned in the [`Err`] returned by
/// [`run_test_with_result`] itself.
///
/// Note: If the test panics, the teardown logic will only be run if the `testing.catch-panic`
/// config is set to `true`. To ensure the teardown logic runs, either set the config or return an
/// error instead of panicking.
pub async fn run_test_with_result<A, S, T, E>(
    app: A,
    options: PrepareOptions,
    // todo: RustRover doesn't seem to recognize `AsyncFnOnce`. Does it just need an update?
    test_fn: T,
) -> Result<(), (Option<crate::error::Error>, Option<E>)>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: 'static + Send + Sync + App<S>,
    T: std::ops::AsyncFnOnce(&TestAppState<A, S>) -> Result<(), E>,
    E: std::error::Error,
{
    let prepared = match app::prepare(app, options).await {
        Ok(prepared) => prepared,
        Err(err) => return Err((Some(err), None)),
    };

    let prepared = PreparedAppWithoutCli {
        app: prepared.app,
        state: prepared.state,
        #[cfg(feature = "db-sql")]
        migrator_registry: prepared.migrator_registry,
        service_registry: prepared.service_registry,
        lifecycle_handler_registry: prepared.lifecycle_handler_registry,
    };

    if let Err(err) = run::before_app(&prepared).await {
        return Err((Some(err), None));
    }

    let pre_run_app_state = TestAppState {
        app: prepared.app,
        state: prepared.state.clone(),
        service_registry: prepared.service_registry,
    };

    tracing::debug!("Starting test");

    let context = AppContext::from_ref(&pre_run_app_state.state);
    let (test_panic, test_result) = if context.config().testing.catch_panic {
        let test_panic = AssertUnwindSafe(test_fn(&pre_run_app_state))
            .catch_unwind()
            .await;
        (Some(test_panic), None)
    } else {
        let test_result = test_fn(&pre_run_app_state).await;
        (None, Some(test_result))
    };

    tracing::debug!("Test complete");

    let after_app_result =
        run::after_app(&prepared.lifecycle_handler_registry, &prepared.state).await;

    let test_result = if let Some(test_panic) = test_panic {
        match test_panic {
            Ok(ok) => Some(ok),
            Err(err) => resume_unwind(err),
        }
    } else {
        test_result
    };

    let test_result = if let Some(Err(err)) = test_result {
        Some(err)
    } else {
        None
    };

    let after_app_result = after_app_result.err();

    if after_app_result.is_some() || test_result.is_some() {
        return Err((after_app_result, test_result));
    }

    Ok(())
}

/// Initialize the app state. Does everything needed to initialize the app state, but does not
/// run any other start up logic, such as running health checks, lifecycle handlers, or services.
///
/// This is intended to only be used to get access to the app's fully set up state in tests.
///
/// This is useful compared to [`run_test`] and [`run_test_with_result`] if you just need
/// access to your app's state and you don't need to run all of your app's startup/teardown logic
/// in your test.
pub async fn test_state<A, S>(app: A, config: AppConfig) -> RoadsterResult<S>
where
    A: 'static + App<S>,
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    let state = prepare::build_state(&app, config).await?;

    let prepared_without_cli = prepare::prepare_without_cli(app, state).await?;

    Ok(prepared_without_cli.state)
}
