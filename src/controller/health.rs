#[cfg(feature = "sidekiq")]
use std::time::Duration;
use std::time::Instant;

#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
#[cfg(feature = "sidekiq")]
use anyhow::bail;
#[cfg(any(feature = "sidekiq", feature = "db-sql"))]
use axum::extract::State;
#[cfg(not(feature = "open-api"))]
use axum::routing::get;
use axum::Json;
#[cfg(not(feature = "open-api"))]
use axum::Router;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(feature = "sidekiq")]
use sidekiq::redis_rs::cmd;
#[cfg(feature = "sidekiq")]
use tokio::time::timeout;
use tracing::instrument;

use crate::app_context::AppContext;
use crate::controller::build_path;
use crate::view::app_error::AppError;

const BASE: &str = "/_health";
#[cfg(feature = "open-api")]
const TAG: &str = "Health";

#[cfg(not(feature = "open-api"))]
pub fn routes<S>(parent: &str) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    Router::new().route(&root, get(health_get::<S>))
}

#[cfg(feature = "open-api")]
pub fn routes<S>(parent: &str) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let root = build_path(parent, BASE);

    ApiRouter::new().api_route(&root, get_with(health_get::<S>, health_get_docs))
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct HeathCheckResponse {
    /// Total latency of checking the health of the app.
    pub latency: u128,
    #[cfg(feature = "db-sql")]
    pub db: ResourceHealth,
    /// Health of the Redis connection used to enqueue Sidekiq jobs.
    #[cfg(feature = "sidekiq")]
    pub redis_enqueue: ResourceHealth,
    /// Health of the Redis connection used to fetch Sidekiq jobs.
    #[cfg(feature = "sidekiq")]
    pub redis_fetch: Option<ResourceHealth>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ResourceHealth {
    status: Status,
    /// How long it takes to acquire a connection from the pool.
    acquire_conn_latency: Option<u128>,
    /// How long it takes to ping the resource after the connection is acquired.
    ping_latency: Option<u128>,
    /// Total latency of checking the health of the resource.
    latency: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ok,
    Err,
}

#[instrument(skip_all)]
async fn health_get<S>(
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))] State(state): State<AppContext<S>>,
) -> Result<Json<HeathCheckResponse>, AppError>
where
    S: Clone + Send + Sync + 'static,
{
    let timer = Instant::now();
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))]
    #[cfg(feature = "db-sql")]
    let db = {
        let db_timer = Instant::now();
        let db_status = if ping_db(state.db()).await.is_ok() {
            Status::Ok
        } else {
            Status::Err
        };
        let db_timer = db_timer.elapsed();
        ResourceHealth {
            status: db_status,
            acquire_conn_latency: None,
            ping_latency: None,
            latency: db_timer.as_millis(),
        }
    };

    #[cfg(feature = "sidekiq")]
    let redis_enqueue = redis_health(state.redis_enqueue()).await;
    #[cfg(feature = "sidekiq")]
    let redis_fetch = if let Some(redis_fetch) = state.redis_fetch() {
        Some(redis_health(redis_fetch).await)
    } else {
        None
    };

    Ok(Json(HeathCheckResponse {
        latency: timer.elapsed().as_millis(),
        #[cfg(feature = "db-sql")]
        db,
        #[cfg(feature = "sidekiq")]
        redis_enqueue,
        #[cfg(feature = "sidekiq")]
        redis_fetch,
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
async fn redis_health(redis: &sidekiq::RedisPool) -> ResourceHealth {
    let redis_timer = Instant::now();
    let (redis_status, acquire_conn_latency, ping_latency) = match ping_redis(redis).await {
        Ok((a, b)) => (Status::Ok, Some(a.as_millis()), Some(b.as_millis())),
        _ => (Status::Err, None, None),
    };
    let redis_timer = redis_timer.elapsed();
    ResourceHealth {
        status: redis_status,
        acquire_conn_latency,
        ping_latency,
        latency: redis_timer.as_millis(),
    }
}

#[cfg(feature = "sidekiq")]
#[instrument(skip_all)]
async fn ping_redis(redis: &sidekiq::RedisPool) -> anyhow::Result<(Duration, Duration)> {
    let timer = Instant::now();
    let mut conn = timeout(Duration::from_secs(5), redis.get()).await??;
    let acquire_conn_latency = timer.elapsed();

    let timer = Instant::now();
    let msg = uuid::Uuid::new_v4().to_string();
    let pong: String = cmd("PING")
        .arg(&msg)
        .query_async(conn.unnamespaced_borrow_mut())
        .await?;
    let ping_latency = timer.elapsed();

    if pong == msg {
        Ok((acquire_conn_latency, ping_latency))
    } else {
        bail!("Ping response does not match input.")
    }
}

#[cfg(feature = "open-api")]
fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                latency: 20,
                #[cfg(feature = "db-sql")]
                db: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: None,
                    ping_latency: None,
                    latency: 10,
                },
                #[cfg(feature = "sidekiq")]
                redis_enqueue: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(5),
                    ping_latency: Some(10),
                    latency: 15,
                },
                #[cfg(feature = "sidekiq")]
                redis_fetch: Some(ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(15),
                    ping_latency: Some(20),
                    latency: 35,
                }),
            })
        })
}
