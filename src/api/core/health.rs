#[cfg(any(feature = "sidekiq", feature = "db-sql"))]
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
#[cfg(feature = "sidekiq")]
use anyhow::anyhow;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(feature = "sidekiq")]
use sidekiq::redis_rs::cmd;
#[cfg(feature = "sidekiq")]
use std::time::Duration;
use std::time::Instant;
#[cfg(feature = "sidekiq")]
use tokio::time::timeout;
use tracing::instrument;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
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
#[non_exhaustive]
pub struct ResourceHealth {
    pub status: Status,
    /// How long it takes to acquire a connection from the pool.
    pub acquire_conn_latency: Option<u128>,
    /// How long it takes to ping the resource after the connection is acquired.
    pub ping_latency: Option<u128>,
    /// Total latency of checking the health of the resource.
    pub latency: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Status {
    Ok,
    Err(ErrorData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ErrorData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

#[instrument(skip_all)]
pub async fn health_check<S>(
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))] state: &AppContext<S>,
) -> RoadsterResult<HeathCheckResponse>
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
            Status::Err(ErrorData { msg: None })
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
    let (redis_enqueue, redis_fetch) = {
        let redis_enqueue = redis_health(state.redis_enqueue());
        if let Some(redis_fetch) = state.redis_fetch() {
            let (redis_enqueue, redis_fetch) =
                tokio::join!(redis_enqueue, redis_health(redis_fetch));
            (redis_enqueue, Some(redis_fetch))
        } else {
            (redis_enqueue.await, None)
        }
    };

    Ok(HeathCheckResponse {
        latency: timer.elapsed().as_millis(),
        #[cfg(feature = "db-sql")]
        db,
        #[cfg(feature = "sidekiq")]
        redis_enqueue,
        #[cfg(feature = "sidekiq")]
        redis_fetch,
    })
}

#[cfg(feature = "db-sql")]
#[instrument(skip_all)]
async fn ping_db(db: &DatabaseConnection) -> RoadsterResult<()> {
    db.ping().await?;
    Ok(())
}

#[cfg(feature = "sidekiq")]
#[instrument(skip_all)]
async fn redis_health(redis: &sidekiq::RedisPool) -> ResourceHealth {
    let redis_timer = Instant::now();
    let (redis_status, acquire_conn_latency, ping_latency) = match ping_redis(redis).await {
        Ok((a, b)) => (Status::Ok, Some(a.as_millis()), Some(b.as_millis())),
        Err(err) => (
            Status::Err(ErrorData {
                msg: Some(err.to_string()),
            }),
            None,
            None,
        ),
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
async fn ping_redis(redis: &sidekiq::RedisPool) -> RoadsterResult<(Duration, Duration)> {
    let timer = Instant::now();
    let mut conn = timeout(Duration::from_secs(1), redis.get()).await??;
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
        Err(anyhow!("Ping response does not match input.").into())
    }
}
