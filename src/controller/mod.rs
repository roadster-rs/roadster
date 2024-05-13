#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(not(feature = "open-api"))]
use axum::Router;
use itertools::Itertools;

#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::app_config::AppConfig;

#[cfg(feature = "open-api")]
pub mod docs;
pub mod health;
pub mod ping;

pub fn build_path(parent: &str, child: &str) -> String {
    // Clean the path to make sure it is valid:
    // 1. Remove any occurrences of double `/`, e.g. `/foo//bar`
    // 2. Remove any trailing `/`
    // 3. Make sure the path starts with a `/`
    let path = format!("{parent}/{child}");
    let path = path.split('/').filter(|s| !s.is_empty()).join("/");
    let path = format!("/{path}");
    path
}

#[cfg(not(feature = "open-api"))]
pub fn default_routes<S>(parent: &str, _config: &AppConfig) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(ping::routes(parent))
        .merge(health::routes(parent))
}

#[cfg(feature = "open-api")]
pub fn default_routes<S>(parent: &str, config: &AppConfig) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    // Todo: Allow disabling the default routes
    ApiRouter::new()
        .merge(ping::routes(parent))
        .merge(health::routes(parent))
        // The docs route is only available when using Aide
        .merge(docs::routes(parent, config))
}
