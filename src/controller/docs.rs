use std::ops::Deref;
use std::sync::Arc;

use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::{Extension, Json};

use crate::config::app_config::AppConfig;
use crate::controller::build_path;

const BASE: &str = "/_docs";
const TAG: &str = "Docs";

/// This API is only available when using Aide.
pub fn routes<S>(parent: &str, config: &AppConfig) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);
    let open_api_schema_path = build_path(&root, "api.json");

    ApiRouter::new()
        .api_route_with(
            &root,
            get_with(
                Scalar::new(&open_api_schema_path)
                    .with_title(&config.app.name)
                    .axum_handler(),
                |op| op.description("Documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
        .api_route_with(
            &build_path(&root, "/redoc"),
            get_with(
                Redoc::new(&open_api_schema_path)
                    .with_title(&config.app.name)
                    .axum_handler(),
                |op| op.description("Redoc documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
        .api_route(
            &open_api_schema_path,
            get_with(docs_get, |op| op.description("OpenAPI schema").tag(TAG)),
        )
}

async fn docs_get(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}
