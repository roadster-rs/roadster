use crate::controller::build_path;
use crate::view::app_error::AppError;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::TransformOperation;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

const BASE: &str = "/health-check";
const TAG: &str = "Health Check";

pub fn routes<S>(parent: &str) -> (Router<S>, ApiRouter<S>)
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    let router = Router::new().route(&root, get(ping_get));
    let api_router = ApiRouter::new().api_route(&root, get_with(ping_get, ping_get_docs));

    (router, api_router)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {}

#[instrument(skip_all)]
async fn ping_get() -> Result<impl IntoApiResponse, AppError> {
    Ok(Json(PingResponse::default()))
}

fn ping_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Ping the server to confirm that it is running.")
        .tag(TAG)
        .response_with::<200, Json<PingResponse>, _>(|res| res.example(PingResponse::default()))
}
