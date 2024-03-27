use crate::controller::build_path;
use crate::view::app_error::AppError;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
#[cfg(not(feature = "open-api"))]
use axum::routing::get;
use axum::Json;
#[cfg(not(feature = "open-api"))]
use axum::Router;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

const BASE: &str = "/_ping";
#[cfg(feature = "open-api")]
const TAG: &str = "Ping";

#[cfg(not(feature = "open-api"))]
pub fn routes<S>(parent: &str) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    Router::new().route(&root, get(ping_get))
}

#[cfg(feature = "open-api")]
pub fn routes<S>(parent: &str) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    ApiRouter::new().api_route(&root, get_with(ping_get, ping_get_docs))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {}

#[instrument(skip_all)]
async fn ping_get() -> Result<Json<PingResponse>, AppError> {
    Ok(Json(PingResponse::default()))
}

#[cfg(feature = "open-api")]
fn ping_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Ping the server to confirm that it is running.")
        .tag(TAG)
        .response_with::<200, Json<PingResponse>, _>(|res| res.example(PingResponse::default()))
}
