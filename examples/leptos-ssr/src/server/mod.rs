use crate::app::App;
use crate::app_state::AppState;
use crate::server::fileserv::file_and_error_handler;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::Router;
use leptos::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use leptos_config::{ConfFile, Env};
use migration::Migrator;
use roadster::app::context::AppContext;
use roadster::app::metadata::AppMetadata;
use roadster::app::App as RoadsterApp;
use roadster::config::environment::Environment;
use roadster::config::AppConfig;
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster::service::registry::ServiceRegistry;

#[cfg(feature = "ssr")]
pub mod fileserv;

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
        let leptos_config = leptos_config(&app_context).await?;
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

async fn leptos_config(context: &AppContext) -> anyhow::Result<ConfFile> {
    // `cargo leptos` runs from the workspace root, so we need to specify this example's
    // `Cargo.toml` even when we run `cargo leptos` form the example's root.
    let mut config = get_configuration(Some("./examples/leptos-ssr/Cargo.toml"))
        .await
        .map_err(|e| anyhow!(e))?;
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
        .leptos_routes(&state, generate_route_list(App), App)
        .fallback(file_and_error_handler)
}
