use std::sync::Arc;

use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

use crate::app_context::AppContext;
use crate::controller::build_path;
use crate::view::app_error::AppError;

const BASE: &str = "/_health";
const TAG: &str = "Health";

pub fn routes<S>(parent: &str) -> (Router<S>, ApiRouter<S>)
where
    S: Clone + Send + Sync + 'static + Into<Arc<AppContext>>,
{
    let root = build_path(parent, BASE);

    let router = Router::new().route(&root, get(health_get::<S>));
    let api_router = ApiRouter::new().api_route(&root, get_with(health_get::<S>, health_get_docs));

    (router, api_router)
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeathCheckResponse {
    pub db: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ok,
    Err,
}

#[instrument(skip_all)]
async fn health_get<S>(State(state): State<S>) -> Result<impl IntoApiResponse, AppError>
where
    S: Clone + Send + Sync + 'static + Into<Arc<AppContext>>,
{
    let state: Arc<AppContext> = state.into();
    let db = if state.db.ping().await.is_ok() {
        Status::Ok
    } else {
        Status::Err
    };
    Ok(Json(HeathCheckResponse { db }))
}

fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse { db: Status::Ok })
        })
}
