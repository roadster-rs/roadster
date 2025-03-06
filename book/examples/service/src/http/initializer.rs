use crate::http::example_b;
use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use roadster::api::http::build_path;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::http::builder::HttpServiceBuilder;
use roadster::service::http::initializer::any::AnyInitializer;
use tracing::info;

const BASE: &str = "/api";

/// Set up the [`HttpServiceBuilder`]. This will then be registered with the
/// [`roadster::service::registry::ServiceRegistry`].
pub async fn http_service(state: &AppContext) -> RoadsterResult<HttpServiceBuilder<AppContext>> {
    HttpServiceBuilder::new(Some(BASE), state)
        .api_router(ApiRouter::new().api_route(
            &build_path(BASE, "/example_b"),
            get_with(example_b::example_b_get, example_b::example_b_get_docs),
        ))
        .initializer(
            AnyInitializer::builder()
                .name("custom-initializer")
                .apply(|router, _state| {
                    info!("Running custom initializer");
                    Ok(router)
                })
                .build(),
        )
}
