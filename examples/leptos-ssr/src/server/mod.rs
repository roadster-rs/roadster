use crate::app::{App, shell};
use crate::app_state::AppState;
use anyhow::anyhow;
use axum::Router;
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::config::environment::Environment;
use roadster::service::http::service::HttpService;

const BASE: &str = "/api";

pub fn build_app() -> RoadsterApp<AppState> {
    RoadsterApp::builder()
        .state_provider(|app_context| {
            let leptos_config = leptos_config(&app_context)?;
            let leptos_options = leptos_config.leptos_options.clone();

            let state = AppState {
                app_context,
                leptos_config,
                leptos_options,
            };
            Ok(state)
        })
        .add_service_provider(|registry, state| {
            Box::pin(async {
                let state = state.clone();
                assert_eq!(
                    state.leptos_options.site_addr,
                    state
                        .app_context
                        .config()
                        .service
                        .http
                        .custom
                        .address
                        .socket_addr()?,
                    "Leptos address does not match the Roadster http address."
                );

                registry
                    .register_builder(
                        HttpService::builder(&state, Some(BASE)).router(leptos_routes(&state)),
                    )
                    .await?;

                Ok(())
            })
        })
        .build()
}

fn leptos_config(context: &AppContext) -> roadster::error::RoadsterResult<ConfFile> {
    let mut config = leptos_config(&context)
        .map_err(|err| roadster::error::other::OtherError::Message(err.to_string().into()))?;
    config.leptos_options.site_addr = context.config().service.http.custom.address.socket_addr()?;
    config.leptos_options.env = match context.config().environment {
        Environment::Production => Env::PROD,
        _ => Env::DEV,
    };
    Ok(config)
}

pub fn leptos_routes(state: &AppState) -> Router<AppState> {
    let state = state.clone();
    Router::<AppState>::new()
        .leptos_routes(&state.clone(), generate_route_list(App), move || {
            shell(state.leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
}
