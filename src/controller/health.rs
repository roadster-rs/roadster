use std::sync::Arc;
use std::time::{Duration, Instant};

use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::transform::TransformOperation;
#[cfg(feature = "sidekiq")]
use anyhow::bail;
#[cfg(any(feature = "sidekiq", feature = "db-sql"))]
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "sidekiq")]
use sidekiq::redis_rs::cmd;
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
    pub ping_latency: u128,
    #[cfg(feature = "db-sql")]
    pub db: ResourceHealth,
    #[cfg(feature = "sidekiq")]
    pub redis: ResourceHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceHealth {
    status: Status,
    ping_latency: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ok,
    Err,
}

#[instrument(skip_all)]
async fn health_get<S>(
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))] State(state): State<S>,
) -> Result<impl IntoApiResponse, AppError>
where
    S: Clone + Send + Sync + 'static + Into<Arc<AppContext>>,
{
    let timer = Instant::now();
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))]
    let state: Arc<AppContext> = state.into();
    #[cfg(feature = "db-sql")]
    let db = {
        let db_timer = Instant::now();
        let db_status = if ping_db(&state.db).await.is_ok() {
            Status::Ok
        } else {
            Status::Err
        };
        let db_timer = db_timer.elapsed();
        ResourceHealth {
            status: db_status,
            ping_latency: db_timer.as_millis(),
        }
    };

    #[cfg(feature = "sidekiq")]
    let redis = {
        let redis_timer = Instant::now();
        let redis_status = match ping_redis(&state.redis).await {
            Ok(_) => Status::Ok,
            _ => Status::Err,
        };
        let redis_timer = redis_timer.elapsed();
        ResourceHealth {
            status: redis_status,
            ping_latency: redis_timer.as_millis(),
        }
    };
    Ok(Json(HeathCheckResponse {
        ping_latency: timer.elapsed().as_millis(),
        #[cfg(feature = "db-sql")]
        db,
        #[cfg(feature = "sidekiq")]
        redis,
    }))
}

#[cfg(feature = "db-sql")]
#[instrument(skip_all)]
async fn ping_db(db: &DatabaseConnection) -> anyhow::Result<()> {
    db.ping().await?;
    Ok(())
}

#[cfg(feature = "sidekiq")]
#[instrument(skip_all)]
async fn ping_redis(redis: &sidekiq::RedisPool) -> anyhow::Result<()> {
    let mut conn = redis.get().await?;
    let msg = uuid::Uuid::new_v4().to_string();
    let pong: String = cmd("PING")
        .arg(&msg)
        .query_async(conn.unnamespaced_borrow_mut())
        .await?;
    if pong == msg {
        Ok(())
    } else {
        bail!("Ping response does not match input.")
    }
}

fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                ping_latency: Duration::from_millis(20).as_millis(),
                #[cfg(feature = "db-sql")]
                db: ResourceHealth {
                    status: Status::Ok,
                    ping_latency: Duration::from_millis(10).as_millis(),
                },
                #[cfg(feature = "sidekiq")]
                redis: ResourceHealth {
                    status: Status::Ok,
                    ping_latency: Duration::from_millis(10).as_millis(),
                },
            })
        })
}
