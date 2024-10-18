use crate::app::context::AppContext;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
use axum::Router;
use axum_core::extract::FromRef;
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

pub fn default_routes<S>(parent: &str, state: &S) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    Router::new()
        .merge(ping::routes(parent, state))
        .merge(health::routes(parent, state))
}

#[cfg(feature = "open-api")]
pub fn default_api_routes<S>(parent: &str, state: &S) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    ApiRouter::new()
        .merge(ping::api_routes(parent, state))
        .merge(health::api_routes(parent, state))
        // The docs route is only available when using Aide
        .merge(docs::routes(parent, state))
}
