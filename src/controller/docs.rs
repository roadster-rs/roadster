use crate::app_context::AppContext;
use crate::controller::build_path;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use std::ops::Deref;
use std::sync::Arc;

const BASE: &str = "/docs";
const TAG: &str = "Docs";

/// This API is only available when using Aide.
pub fn routes<S>(parent: &str, context: &AppContext) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    ApiRouter::new()
        .api_route_with(
            &root,
            get_with(
                Scalar::new("/api/docs/api.json")
                    .with_title(&context.config.app.name)
                    .axum_handler(),
                |op| op.description("Documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
        .api_route_with(
            &build_path(&root, "/redoc"),
            get_with(
                Redoc::new("/api/docs/api.json")
                    .with_title(&context.config.app.name)
                    .axum_handler(),
                |op| op.description("Redoc documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
        .api_route(
            &build_path(&root, "/api.json"),
            get_with(docs_get, |op| op.description("OpenAPI spec").tag(TAG)),
        )
}

async fn docs_get(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}
