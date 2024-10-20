use crate::app::{shell, App};
use crate::app_state::AppState;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::Router;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use migration::Migrator;
use roadster::app::context::AppContext;
use roadster::app::metadata::AppMetadata;
use roadster::app::App as RoadsterApp;
use roadster::config::environment::Environment;
use roadster::config::AppConfig;
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster::service::registry::ServiceRegistry;

const BASE: &str = "/api";

#[derive(Default)]
pub struct Server;

#[async_trait]
impl RoadsterApp<AppState> for Server {
    type Cli = crate::cli::AppCli;
    type M = Migrator;

    fn metadata(&self, _config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(AppMetadata::builder()
            .version(env!("VERGEN_GIT_SHA").to_string())
            .build())
    }

    async fn provide_state(&self, app_context: AppContext) -> RoadsterResult<AppState> {
        let leptos_config = leptos_config(&app_context)?;
        let leptos_options = leptos_config.leptos_options.clone();

        let state = AppState {
            app_context,
            leptos_config,
            leptos_options,
        };
        Ok(state)
    }

    async fn services(
        &self,
        registry: &mut ServiceRegistry<Self, AppState>,
        state: &AppState,
    ) -> RoadsterResult<()> {
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
                HttpService::builder(Some(BASE), &state).router(leptos_routes(&state)),
            )
            .await?;

        Ok(())
    }
}

fn leptos_config(context: &AppContext) -> anyhow::Result<ConfFile> {
    let mut config =
        get_configuration(Some("./examples/leptos-0.7-ssr/Cargo.toml")).map_err(|e| anyhow!(e))?;
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
