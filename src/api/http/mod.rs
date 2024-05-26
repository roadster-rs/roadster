use crate::app_context::AppContext;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
use axum::Router;
use itertools::Itertools;

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

pub fn default_routes<S>(parent: &str, context: &AppContext<S>) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .merge(ping::routes(parent, context))
        .merge(health::routes(parent, context))
}

#[cfg(feature = "open-api")]
pub fn default_api_routes<S>(parent: &str, context: &AppContext<S>) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    ApiRouter::new()
        .merge(ping::api_routes(parent, context))
        .merge(health::api_routes(parent, context))
        // The docs route is only available when using Aide
        .merge(docs::routes(parent, context))
}
