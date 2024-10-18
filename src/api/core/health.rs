use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health_check::{CheckResponse, ErrorData, HealthCheck, Status};
#[cfg(feature = "open-api")]
use aide::OperationIo;
#[cfg(any(feature = "sidekiq", feature = "email-smtp"))]
use anyhow::anyhow;
use axum_core::extract::FromRef;
use futures::future::join_all;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(feature = "sidekiq")]
use sidekiq::redis_rs::cmd;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema, OperationIo))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HeathCheckResponse {
    /// Total latency of checking the health of the app.
    pub latency: u128,
    pub resources: BTreeMap<String, CheckResponse>,
}

#[instrument(skip_all)]
pub async fn health_check<S>(
    state: &S,
    duration: Option<Duration>,
) -> RoadsterResult<HeathCheckResponse>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    if let Some(duration) = duration.as_ref() {
        info!(
            "Running checks for a maximum duration of {} ms",
            duration.as_millis()
        );
    } else {
        info!("Running checks");
    }
    let context = AppContext::from_ref(state);
    let timer = Instant::now();

    let check_futures = context.health_checks().into_iter().map(|check| {
        Box::pin(async move {
            let name = check.name();
            info!(%name, "Running check");
            let check_timer = Instant::now();
            let result = match run_check(check, duration).await {
                Ok(response) => {
                    info!(%name, latency_ms=%response.latency, "Check completed");
                    match &response.status {
                        Status::Ok => {}
                        Status::Err(_) => {
                            error!(%name, "Resource is not healthy");
                            debug!(%name, "Error details: {response:?}");
                        }
                    }
                    response
                }
                Err(err) => CheckResponse::builder()
                    .status(Status::Err(
                        ErrorData::builder()
                            .msg(format!(
                                "An error occurred while running health check `{name}`: {err}"
                            ))
                            .build(),
                    ))
                    .latency(check_timer.elapsed())
                    .build(),
            };
            (name, result)
        })
    });

    let resources = join_all(check_futures).await.into_iter().collect();

    let latency = timer.elapsed().as_millis();

    info!(latency_ms=%latency, "Checks completed");

    Ok(HeathCheckResponse { latency, resources })
}

async fn run_check(
    check: Arc<dyn HealthCheck>,
    duration: Option<Duration>,
) -> RoadsterResult<CheckResponse> {
    if let Some(duration) = duration {
        timeout(duration, check.check()).await?
    } else {
        check.check().await
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Latency {
    /// How long it takes to acquire a connection from the pool.
    pub acquire_conn_latency: Option<u128>,
    /// How long it takes to ping the resource after the connection is acquired.
    pub ping_latency: Option<u128>,
}

#[cfg(feature = "db-sql")]
pub(crate) async fn db_health(context: &AppContext, duration: Option<Duration>) -> CheckResponse {
    let db_timer = Instant::now();
    let db_status = match ping_db(context.db(), duration).await {
        Ok(_) => Status::Ok,
        Err(err) => Status::Err(ErrorData::builder().msg(err.to_string()).build()),
    };
    let db_timer = db_timer.elapsed();
    CheckResponse::builder()
        .status(db_status)
        .latency(db_timer)
        .build()
}

#[cfg(feature = "db-sql")]
#[instrument(skip_all)]
async fn ping_db(db: &DatabaseConnection, duration: Option<Duration>) -> RoadsterResult<()> {
    if let Some(duration) = duration {
        timeout(duration, db.ping()).await??;
    } else {
        db.ping().await?;
    }
    Ok(())
}

#[cfg(feature = "email-smtp")]
pub(crate) async fn smtp_health(context: &AppContext, duration: Option<Duration>) -> CheckResponse {
    let timer = Instant::now();
    let status = match ping_smtp(context.mailer(), duration).await {
        Ok(_) => Status::Ok,
        Err(err) => Status::Err(ErrorData::builder().msg(err.to_string()).build()),
    };
    let timer = timer.elapsed();
    CheckResponse::builder()
        .status(status)
        .latency(timer)
        .build()
}

#[cfg(feature = "email-smtp")]
async fn ping_smtp(
    mailer: &lettre::SmtpTransport,
    duration: Option<Duration>,
) -> RoadsterResult<()> {
    let connected = if let Some(duration) = duration {
        timeout(duration, async { mailer.test_connection() }).await??
    } else {
        mailer.test_connection()?
    };
    if connected {
        Ok(())
    } else {
        Err(anyhow!("Not connected to the SMTP server").into())
    }
}

#[cfg(feature = "sidekiq")]
#[instrument(skip_all)]
pub(crate) async fn redis_health(
    redis: &sidekiq::RedisPool,
    duration: Option<Duration>,
) -> CheckResponse {
    let redis_timer = Instant::now();
    let (redis_status, acquire_conn_latency, ping_latency) = match ping_redis(redis, duration).await
    {
        Ok((a, b)) => (Status::Ok, Some(a.as_millis()), Some(b.as_millis())),
        Err(err) => (
            Status::Err(ErrorData::builder().msg(err.to_string()).build()),
            None,
            None,
        ),
    };
    let redis_timer = redis_timer.elapsed();
    CheckResponse::builder()
        .status(redis_status)
        .latency(redis_timer)
        .custom(Latency {
            acquire_conn_latency,
            ping_latency,
        })
        .build()
}

#[cfg(feature = "sidekiq")]
#[instrument(skip_all)]
async fn ping_redis(
    redis: &sidekiq::RedisPool,
    duration: Option<Duration>,
) -> RoadsterResult<(Duration, Duration)> {
    let timer = Instant::now();
    let mut conn = if let Some(duration) = duration {
        timeout(duration, redis.get()).await??
    } else {
        redis.get().await?
    };
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
