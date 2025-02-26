#[cfg(feature = "cli")]
use crate::api::cli::CliState;
use crate::app::context::AppContext;
use crate::app::prepare::{PrepareOptions, PreparedApp, PreparedAppWithoutCli};
use crate::app::{App, prepare};
use crate::error::RoadsterResult;
use crate::lifecycle::registry::LifecycleHandlerRegistry;
use axum_core::extract::FromRef;
use tracing::{error, info, warn};

/// Run the [`App`]
pub async fn run<A, S>(app: A) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Send + Sync + 'static,
{
    let cli_and_state =
        prepare::build_cli_and_state(app, PrepareOptions::builder().build()).await?;

    let prepared = prepare::prepare_from_cli_and_state(cli_and_state).await?;

    run_prepared(prepared).await?;

    Ok(())
}

/// Run a [PreparedApp] that was previously crated by [prepare]
pub async fn run_prepared<A, S>(prepared: PreparedApp<A, S>) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[cfg(feature = "cli")]
    let prepared = {
        if let Some(cli) = prepared.cli {
            let cli = CliState {
                roadster_cli: cli.roadster_cli,
                app_cli: cli.app_cli,
                app: prepared.app,
                state: prepared.state,
                #[cfg(feature = "db-sql")]
                migrators: prepared.migrators,
                service_registry: prepared.service_registry,
            };
            if crate::api::cli::handle_cli(&cli).await? {
                return Ok(());
            }
            PreparedAppWithoutCli {
                app: cli.app,
                state: cli.state,
                #[cfg(feature = "db-sql")]
                migrators: cli.migrators,
                service_registry: cli.service_registry,
                lifecycle_handler_registry: prepared.lifecycle_handler_registry,
            }
        } else {
            PreparedAppWithoutCli {
                app: prepared.app,
                state: prepared.state,
                #[cfg(feature = "db-sql")]
                migrators: prepared.migrators,
                service_registry: prepared.service_registry,
                lifecycle_handler_registry: prepared.lifecycle_handler_registry,
            }
        }
    };

    #[cfg(not(feature = "cli"))]
    let prepared = PreparedAppWithoutCli {
        app: prepared.app,
        state: prepared.state,
        #[cfg(feature = "db-sql")]
        migrators: prepared.migrators,
        service_registry: prepared.service_registry,
        lifecycle_handler_registry: prepared.lifecycle_handler_registry,
    };

    run_prepared_without_cli(prepared).await
}

/// Run a [`PreparedApp`] that was previously crated by [`prepare`] without handling CLI commands
/// (they should have been handled already).
async fn run_prepared_without_cli<A, S>(prepared: PreparedAppWithoutCli<A, S>) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    before_app(&prepared).await?;

    let result =
        crate::service::runner::run(prepared.app, prepared.service_registry, &prepared.state).await;
    if let Err(err) = result {
        error!("An error occurred in the app: {err}");
    }

    after_app(&prepared.lifecycle_handler_registry, &prepared.state).await?;

    Ok(())
}

/// Run the app's initialization logic (lifecycle handlers, health checks, etc).
pub(crate) async fn before_app<A, S>(
    prepared_app: &PreparedAppWithoutCli<A, S>,
) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    if prepared_app.service_registry.services.is_empty() {
        warn!("No enabled services were registered.");
    }

    let lifecycle_handlers = prepared_app
        .lifecycle_handler_registry
        .handlers(&prepared_app.state);

    info!("Running AppLifecycleHandler::before_health_checks");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_health_checks");
        handler.before_health_checks(prepared_app).await?;
    }

    let context = AppContext::from_ref(&prepared_app.state);
    crate::service::runner::health_checks(context.health_checks()).await?;

    info!("Running AppLifecycleHandler::before_services");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_services");
        handler.before_services(prepared_app).await?
    }
    crate::service::runner::before_run(&prepared_app.service_registry, &prepared_app.state).await?;

    Ok(())
}

/// Run the app's teardown logic.
pub async fn after_app<A, S>(
    lifecycle_handler_registry: &LifecycleHandlerRegistry<A, S>,
    state: &S,
) -> RoadsterResult<()>
where
    A: App<S> + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    info!("Shutting down");

    let lifecycle_handlers = lifecycle_handler_registry.handlers(state);

    info!("Running AppLifecycleHandler::before_shutdown");
    for handler in lifecycle_handlers.iter() {
        info!(name=%handler.name(), "Running AppLifecycleHandler::before_shutdown");
        let result = handler.on_shutdown(state).await;
        if let Err(err) = result {
            error!(name=%handler.name(), "An error occurred when running AppLifecycleHandler::before_shutdown: {err}");
        }
    }

    // Todo: Move to a lifecycle handler? Currently this is only used for tests so it's probably
    //  okay to keep it here for now.
    #[cfg(feature = "testing")]
    {
        let context = AppContext::from_ref(state);
        context.teardown().await?;
    }

    info!("Shutdown complete");

    Ok(())
}
