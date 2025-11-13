use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::{Json, Router};
use roadster::api::http::build_path;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::http::builder::HttpServiceBuilder;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const BASE: &str = "/api";

pub fn http_service(state: &AppContext) -> HttpServiceBuilder<AppContext> {
    HttpServiceBuilder::new(state, Some(BASE))
        // Create your routes as an `ApiRouter` in order to include it in the OpenAPI schema.
        .api_router(
            ApiRouter::new()
                // Register a `GET` route on the `ApiRouter`
                .api_route(
                    &build_path(BASE, "/example"),
                    get_with(example_get, example_get_docs),
                ),
        )
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleResponse {}

#[instrument(skip_all)]
pub async fn example_get(
    State(_state): State<AppContext>,
) -> RoadsterResult<Json<ExampleResponse>> {
    Ok(Json(ExampleResponse {}))
}

pub fn example_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example API.")
        .tag("Example")
        .response_with::<200, Json<ExampleResponse>, _>(|res| res.example(ExampleResponse {}))
}
