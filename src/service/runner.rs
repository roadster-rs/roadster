use crate::api::core::health::health_check_with_checks;
use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health::check::HealthCheck;
use crate::health::check::Status;
use crate::service::registry::ServiceRegistry;
use axum_core::extract::FromRef;
use itertools::Itertools;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

#[instrument(skip_all)]
pub(crate) async fn health_checks(
    checks: Vec<Arc<dyn HealthCheck<Error = crate::error::Error>>>,
) -> RoadsterResult<()> {
    let duration = Duration::from_secs(60);
    let response = health_check_with_checks(checks, Some(duration)).await?;

    let error_responses = response
        .resources
        .iter()
        .filter(|(_name, response)| !matches!(response.status, Status::Ok))
        .collect_vec();

    if error_responses.is_empty() {
        Ok(())
    } else {
        let names = error_responses.iter().map(|(name, _)| name).collect_vec();
        Err(crate::error::other::OtherError::Message(
            format!("Health checks failed: {names:?}").into(),
        )
        .into())
    }
}

#[instrument(skip_all)]
pub(crate) async fn before_run<S>(
    state: &S,
    service_registry: &ServiceRegistry<S>,
) -> RoadsterResult<()>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    for (_, service) in service_registry.services.iter() {
        let name = service.name();
        info!(service.name = name, "Running service::before_run");
        service.before_run(state).await?;
    }

    Ok(())
}

pub(crate) async fn run<A, S>(
    app: A,
    state: &S,
    service_registry: ServiceRegistry<S>,
) -> RoadsterResult<()>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: 'static + App<S>,
{
    let app = Arc::new(app);
    let cancel_token = CancellationToken::new();
    let mut join_set = JoinSet::new();

    let context = AppContext::from_ref(state);

    // Spawn tasks for the app's services
    for service in service_registry.services.into_values() {
        if !service.enabled(state).await {
            continue;
        }
        let name = service.name();
        let state = state.clone();
        let cancel_token = cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            info!(service.name = name, "Running service");
            service.run(&state, cancel_token).await
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
                    cancel_on_error(cancel_token.clone(), context.clone());
                }
            }
            Err(join_err) => {
                error!(
                    "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
                );
                cancel_on_error(cancel_token.clone(), context.clone());
            }
        }
    }

    Ok(())
}

async fn graceful_shutdown_signal<F>(cancellation_token: CancellationToken, app_shutdown_signal: F)
where
    F: 'static + Send + Future<Output = ()>,
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
    F: 'static + Send + Future<Output = ()>,
{
    shutdown_signal.await;
    cancellation_token.cancel();
    Ok(())
}

fn cancel_on_error(cancellation_token: CancellationToken, context: AppContext) {
    if context.config().app.shutdown_on_error {
        warn!("Cancelling other tasks");
        cancellation_token.cancel();
    }
}
