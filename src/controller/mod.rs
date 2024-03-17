use crate::app_context::AppContext;
use aide::axum::ApiRouter;
use axum::Router;
use itertools::Itertools;

pub mod docs;
pub mod middleware;
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

pub fn default_routes<S>(parent: &str, context: &AppContext) -> (Router<S>, ApiRouter<S>)
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new().merge(ping::routes(parent).0);

    let api_router = ApiRouter::new()
        .merge(ping::routes(parent).1)
        // The docs route is only available when using Aide
        .merge(docs::routes(parent, context));

    (router, api_router)
}
