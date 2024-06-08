#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::service::registry::ServiceRegistry;
use std::future::Future;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};

#[cfg(feature = "cli")]
pub(crate) async fn handle_cli<A>(
    roadster_cli: &RoadsterCli,
    app_cli: &A::Cli,
    service_registry: &ServiceRegistry<A>,
    context: &AppContext<A::State>,
) -> RoadsterResult<bool>
where
    A: App,
{
    for (_name, service) in service_registry.services.iter() {
        if service.handle_cli(roadster_cli, app_cli, context).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) async fn run<A>(
    service_registry: ServiceRegistry<A>,
    context: &AppContext<A::State>,
) -> RoadsterResult<()>
where
    A: App,
{
    let cancel_token = CancellationToken::new();
    let mut join_set = JoinSet::new();

    // Spawn tasks for the app's services
    for (name, service) in service_registry.services {
        let context = context.clone();
        let cancel_token = cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            info!(service=%name, "Running service");
            service.run(&context, cancel_token).await
        }));
    }

    // Task to clean up resources when gracefully shutting down.
    {
        let context = context.clone();
        let cancel_token = cancel_token.clone();
        let app_graceful_shutdown = {
            let context = context.clone();
            Box::pin(async move { A::graceful_shutdown(&context).await })
        };
        join_set.spawn(Box::pin(async move {
            cancel_on_error(
                cancel_token.clone(),
                &context,
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
        let context = context.clone();
        let app_graceful_shutdown_signal = {
            let context = context.clone();
            Box::pin(async move { A::graceful_shutdown_signal(&context).await })
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

    info!("Shutdown complete");

    Ok(())
}

async fn graceful_shutdown_signal<F>(cancellation_token: CancellationToken, app_shutdown_signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
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

async fn cancel_on_error<T, F, S>(
    cancellation_token: CancellationToken,
    context: &AppContext<S>,
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
async fn graceful_shutdown<F1, F2, S>(
    shutdown_signal: F1,
    app_graceful_shutdown: F2,
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] context: AppContext<S>,
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
    info!("Running app's custom shutdown logic.");
    let app_graceful_shutdown_result = app_graceful_shutdown.await;

    #[cfg(feature = "db-sql")]
    db_close_result?;
    app_graceful_shutdown_result?;

    Ok(())
}
