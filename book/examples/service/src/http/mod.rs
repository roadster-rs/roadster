mod example_b;
mod example_c;
mod open_api;

use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use axum::Router;
use axum::response::IntoResponse;
use axum::routing::get;
use roadster::api::http::build_path;
use roadster::app::context::AppContext;
use roadster::service::http::builder::HttpServiceBuilder;

const BASE: &str = "/api";

/// Set up the [`HttpServiceBuilder`]. This will then be registered with the
/// [`roadster::service::registry::ServiceRegistry`].
pub async fn http_service(state: &AppContext) -> HttpServiceBuilder<AppContext> {
    HttpServiceBuilder::new(Some(BASE), state)
        // Multiple routers can be registered and they will all be merged together using the
        // `axum::Router::merge` method.
        .router(Router::new().route(&build_path(BASE, "/example_a"), get(example_a)))
        // Create your routes as an `ApiRouter` in order to include it in the OpenAPI schema.
        .api_router(ApiRouter::new().api_route(
            &build_path(BASE, "/example_b"),
            get_with(example_b::example_b_get, example_b::example_b_get_docs),
        ))
        .api_router(ApiRouter::new().api_route(
            &build_path(BASE, "/example_c"),
            get_with(example_c::example_c_get, example_c::example_c_get_docs),
        ))
}

async fn example_a() -> impl IntoResponse {
    ()
}
