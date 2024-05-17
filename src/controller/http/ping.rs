#[mockall_double::double]
use crate::app_context::AppContext;
use crate::controller::http::build_path;
use crate::view::http::app_error::AppError;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
use axum::routing::get;
use axum::Json;
use axum::Router;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

#[cfg(feature = "open-api")]
const TAG: &str = "Ping";

pub fn routes<S>(parent: &str, context: &AppContext<S>) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new();
    if !enabled(context) {
        return router;
    }
    let root = build_path(parent, route(context));
    router.route(&root, get(ping_get))
}

#[cfg(feature = "open-api")]
pub fn api_routes<S>(parent: &str, context: &AppContext<S>) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let router = ApiRouter::new();
    if !enabled(context) {
        return router;
    }
    let root = build_path(parent, route(context));
    router.api_route(&root, get_with(ping_get, ping_get_docs))
}

fn enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .ping
        .enabled(context)
}

fn route<S>(context: &AppContext<S>) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .ping
        .route
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
