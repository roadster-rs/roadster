use crate::app::App;
use crate::app_state::AppState;
use crate::server::fileserv::file_and_error_handler;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::Router;
use leptos::get_configuration;
use leptos_axum::{generate_route_list, LeptosRoutes};
use migration::Migrator;
use roadster::app::context::AppContext;
use roadster::app::metadata::AppMetadata;
use roadster::app::App as RoadsterApp;
use roadster::config::app_config::AppConfig;
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

    fn metadata(_config: &AppConfig) -> RoadsterResult<AppMetadata> {
        Ok(AppMetadata::builder()
            .version(env!("VERGEN_GIT_SHA").to_string())
            .build())
    }

    async fn provide_state(app_context: AppContext) -> RoadsterResult<AppState> {
        let leptos_config = get_configuration(None).await.map_err(|e| anyhow!(e))?;
        let leptos_options = leptos_config.leptos_options.clone();
        let state = AppState {
            app_context,
            leptos_config,
            leptos_options,
        };
        Ok(state)
    }

    async fn services(
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
        let routes = generate_route_list(App);
        registry
            .register_builder(
                HttpService::builder(Some(BASE), &state.clone()).router(
                    Router::<AppState>::new()
                        .leptos_routes(&state, routes, App)
                        .fallback(file_and_error_handler),
                ),
            )
            .await?;

        Ok(())
    }
}
