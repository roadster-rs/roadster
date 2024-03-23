use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use aide::axum::{ApiRouter, IntoApiResponse};
use aide::axum::routing::get_with;
use aide::transform::TransformOperation;
use axum::{Json, Router};
use axum::extract::State;
use axum::routing::get;
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use sidekiq::redis_rs::cmd;
use sidekiq::Worker;
use tracing::instrument;

use crate::app::Foo;
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
    pub redis: Option<Status>,
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
    // Foo::perform_async(state.redis.as_ref().unwrap(), ()).await?;
    let db = if state.db.ping().await.is_ok() {
        Status::Ok
    } else {
        Status::Err
    };
    let redis = if let Some(redis) = state.redis.as_ref() {
        let mut conn = redis.get().await?;
        let pong = cmd("PING")
            .query_async(conn.unnamespaced_borrow_mut())
            .await?;
        Some(Status::Ok)
    } else {
        None
    };
    Ok(Json(HeathCheckResponse { db, redis }))
}

fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                db: Status::Ok,
                redis: Status::Ok,
            })
        })
}
