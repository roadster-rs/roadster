#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
use crate::api::core::health::health_check;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::health_check::Status;
use crate::service::registry::ServiceRegistry;
use anyhow::anyhow;
use axum::extract::FromRef;
use itertools::Itertools;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

#[cfg(feature = "cli")]
#[instrument(skip_all)]
pub(crate) async fn handle_cli<A, S>(
    roadster_cli: &RoadsterCli,
    app_cli: &A::Cli,
    service_registry: &ServiceRegistry<A, S>,
    state: &S,
) -> RoadsterResult<bool>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    for (_name, service) in service_registry.services.iter() {
        if service.handle_cli(roadster_cli, app_cli, state).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

#[instrument(skip_all)]
pub(crate) async fn health_checks(context: &AppContext) -> RoadsterResult<()> {
    let duration = Duration::from_secs(60);
    info!(
        "Running checks for a maximum duration of {} seconds",
        duration.as_secs()
    );
    let response = health_check(context, Some(duration)).await?;

    let error_responses = response
        .resources
        .iter()
        .filter(|(_name, response)| !matches!(response.status, Status::Ok))
        .collect_vec();

    error_responses.iter().for_each(|(name, response)| {
        error!(%name, "Resource is not healthy");
        debug!(%name, "Error details: {response:?}");
    });

    if error_responses.is_empty() {
        Ok(())
    } else {
        let names = error_responses.iter().map(|(name, _)| name).collect_vec();
        Err(anyhow!("Health checks failed: {names:?}"))?
    }
}

#[instrument(skip_all)]
pub(crate) async fn before_run<A, S>(
    service_registry: &ServiceRegistry<A, S>,
    state: &S,
) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    for (name, service) in service_registry.services.iter() {
        info!(%name, "Running service::before_run");
        service.before_run(state).await?;
    }

    Ok(())
}

pub(crate) async fn run<A, S>(
    app: A,
    service_registry: ServiceRegistry<A, S>,
    state: &S,
) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let app = Arc::new(app);
    let cancel_token = CancellationToken::new();
    let mut join_set = JoinSet::new();

    // Spawn tasks for the app's services
    for (name, service) in service_registry.services {
        let context = state.clone();
        let cancel_token = cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            info!(%name, "Running service");
            service.run(&context, cancel_token).await
        }));
    }

    // Task to clean up resources when gracefully shutting down.
    {
        let cancel_token = cancel_token.clone();
        let app_graceful_shutdown = {
            let state = state.clone();
            let app = app.clone();
            Box::pin(async move { app.graceful_shutdown(&state).await })
        };
        let context = AppContext::from_ref(state);
        join_set.spawn(Box::pin(async move {
            cancel_on_error(
                cancel_token.clone(),
                context.clone(),
                graceful_shutdown(
                    token_shutdown_signal(cancel_token.clone()),
                    app_graceful_shutdown,
                    context.clone(),
                ),
            )
            .await
        }));
    }
    // Task to listen for the signal to gracefully shutdown, and trigger other tasks to stop.
    {
        let app_graceful_shutdown_signal = {
            let context = state.clone();
            let app = app.clone();
            Box::pin(async move { app.graceful_shutdown_signal(&context).await })
        };
        let graceful_shutdown_signal =
            graceful_shutdown_signal(cancel_token.clone(), app_graceful_shutdown_signal);
        join_set.spawn(cancel_token_on_signal_received(
            graceful_shutdown_signal,
            cancel_token.clone(),
        ));
    }

    // Wait for all the tasks to complete.
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(join_ok) => {
                if let Err(err) = join_ok {
                    error!("An error occurred in one of the app's tasks. Error: {err}");
                }
            }
            Err(join_err) => {
                error!(
                    "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
                );
            }
        }
    }

    Ok(())
}

async fn graceful_shutdown_signal<F>(cancellation_token: CancellationToken, app_shutdown_signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    #[allow(clippy::expect_used)]
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    #[allow(clippy::expect_used)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Shutting down due to ctrl-c signal received");
        },
        _ = sigterm => {
            info!("Shutting down due to sigterm signal received");
        },
        _ = cancellation_token.cancelled() => {
            info!("Shutting down due to cancellation token cancelled");
        }
        _ = app_shutdown_signal => {
            info!("Shutting down due to app's custom shutdown signal received");
        }
    }
}

async fn cancel_token_on_signal_received<F>(
    shutdown_signal: F,
    cancellation_token: CancellationToken,
) -> RoadsterResult<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    shutdown_signal.await;
    cancellation_token.cancel();
    Ok(())
}

async fn token_shutdown_signal(cancellation_token: CancellationToken) {
    cancellation_token.cancelled().await
}

async fn cancel_on_error<T, F>(
    cancellation_token: CancellationToken,
    context: AppContext,
    f: F,
) -> RoadsterResult<T>
where
    F: Future<Output = RoadsterResult<T>> + Send + 'static,
{
    let result = f.await;
    if result.is_err() && context.config().app.shutdown_on_error {
        cancellation_token.cancel();
    }
    result
}

#[instrument(skip_all)]
async fn graceful_shutdown<F1, F2>(
    shutdown_signal: F1,
    app_graceful_shutdown: F2,
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] context: AppContext,
) -> RoadsterResult<()>
where
    F1: Future<Output = ()> + Send + 'static,
    F2: Future<Output = RoadsterResult<()>> + Send + 'static,
{
    shutdown_signal.await;

    info!("Received shutdown signal. Shutting down gracefully.");

    #[cfg(feature = "db-sql")]
    let db_close_result = {
        info!("Closing the DB connection pool.");
        context.db().clone().close().await
    };

    // Futures are lazy -- the custom `app_graceful_shutdown` future won't run until we call `await` on it.
    // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
    info!("Running App::graceful_shutdown.");
    let app_graceful_shutdown_result = app_graceful_shutdown.await;

    #[cfg(feature = "db-sql")]
    db_close_result?;
    app_graceful_shutdown_result?;

    Ok(())
}
