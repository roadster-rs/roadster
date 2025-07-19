use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health::check::{CheckResponse, ErrorData, HealthCheck, Status};
#[cfg(feature = "open-api")]
use aide::OperationIo;
use axum_core::extract::FromRef;
use futures::future::join_all;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
#[cfg(feature = "db-sea-orm")]
use sea_orm::DatabaseConnection;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(feature = "worker-sidekiq")]
use sidekiq::redis_rs::cmd;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument};

#[serde_with::skip_serializing_none]
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
    let context = AppContext::from_ref(state);
    health_check_with_checks(context.health_checks(), duration).await
}

#[instrument(skip_all)]
pub(crate) async fn health_check_with_checks<S>(
    checks: Vec<Arc<dyn HealthCheck>>,
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
    let timer = Instant::now();

    let check_futures = checks.into_iter().map(|check| {
        Box::pin(async move {
            let name = check.name();
            info!(health_check.name = name, "Running check");
            let check_timer = Instant::now();
            let result = match run_check(check, duration).await {
                Ok(response) => {
                    info!(health_check.name = name, latency_ms=%response.latency, "Check completed");
                    match &response.status {
                        Status::Ok => {}
                        Status::Err(_) => {
                            error!(health_check.name = name, "Resource is not healthy");
                            debug!(health_check.name = name, "Error details: {response:?}");
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

#[cfg(feature = "db-sea-orm")]
pub(crate) async fn db_sea_orm_health(
    context: &AppContext,
    duration: Option<Duration>,
) -> CheckResponse {
    let db_timer = Instant::now();
    let db_status = match ping_db_sea_orm(context.sea_orm(), duration).await {
        Ok(_) => Status::Ok,
        Err(err) => Status::Err(ErrorData::builder().msg(err.to_string()).build()),
    };
    let db_timer = db_timer.elapsed();
    CheckResponse::builder()
        .status(db_status)
        .latency(db_timer)
        .build()
}

#[cfg(feature = "db-sea-orm")]
#[instrument(skip_all)]
async fn ping_db_sea_orm(
    db: &DatabaseConnection,
    duration: Option<Duration>,
) -> RoadsterResult<()> {
    if let Some(duration) = duration {
        timeout(duration, db.ping()).await??;
    } else {
        db.ping().await?;
    }
    Ok(())
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-pool")]
pub(crate) async fn db_diesel_health<C>(
    pool: &r2d2::Pool<diesel::r2d2::ConnectionManager<C>>,
    duration: Option<Duration>,
) -> CheckResponse
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
{
    let db_timer = Instant::now();
    let (db_status, acquire_conn_latency, ping_latency) = match ping_diesel_db(pool, duration) {
        Ok((acquire_latency, ping_latency)) => (
            Status::Ok,
            Some(acquire_latency.as_millis()),
            Some(ping_latency.as_millis()),
        ),
        Err(err) => (
            Status::Err(ErrorData::builder().msg(err.to_string()).build()),
            None,
            None,
        ),
    };
    let db_timer = db_timer.elapsed();
    CheckResponse::builder()
        .status(db_status)
        .latency(db_timer)
        .custom(Latency {
            acquire_conn_latency,
            ping_latency,
        })
        .build()
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-pool")]
#[instrument(skip_all)]
fn ping_diesel_db<C>(
    pool: &r2d2::Pool<diesel::r2d2::ConnectionManager<C>>,
    duration: Option<Duration>,
) -> RoadsterResult<(Duration, Duration)>
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
{
    let timer = Instant::now();
    let mut conn = if let Some(duration) = duration {
        pool.get_timeout(duration)?
    } else {
        pool.get()?
    };
    let acquire_conn_latency = timer.elapsed();

    let timer = Instant::now();
    conn.ping()?;
    let ping_latency = timer.elapsed();

    Ok((acquire_conn_latency, ping_latency))
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-postgres-pool-async")]
pub(crate) async fn db_diesel_health_pg_async(
    context: &AppContext,
    duration: Option<Duration>,
) -> CheckResponse {
    let db_timer = Instant::now();
    let (db_status, acquire_conn_latency, ping_latency) =
        match ping_diesel_db_pg_async(context, duration).await {
            Ok((acquire_latency, ping_latency)) => (
                Status::Ok,
                Some(acquire_latency.as_millis()),
                Some(ping_latency.as_millis()),
            ),
            Err(err) => (
                Status::Err(ErrorData::builder().msg(err.to_string()).build()),
                None,
                None,
            ),
        };
    let db_timer = db_timer.elapsed();
    CheckResponse::builder()
        .status(db_status)
        .latency(db_timer)
        .custom(Latency {
            acquire_conn_latency,
            ping_latency,
        })
        .build()
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-postgres-pool-async")]
#[instrument(skip_all)]
async fn ping_diesel_db_pg_async(
    context: &AppContext,
    duration: Option<Duration>,
) -> RoadsterResult<(Duration, Duration)> {
    use diesel_async::pooled_connection::PoolableConnection;

    let timer = Instant::now();
    let mut conn = if let Some(duration) = duration {
        timeout(duration, context.diesel_pg_pool_async().get()).await??
    } else {
        context.diesel_pg_pool_async().get().await?
    };
    let acquire_conn_latency = timer.elapsed();

    let timer = Instant::now();
    conn.ping(&diesel_async::pooled_connection::RecyclingMethod::Fast)
        .await?;
    let ping_latency = timer.elapsed();

    Ok((acquire_conn_latency, ping_latency))
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-mysql-pool-async")]
pub(crate) async fn db_diesel_health_mysql_async(
    context: &AppContext,
    duration: Option<Duration>,
) -> CheckResponse {
    let db_timer = Instant::now();
    let (db_status, acquire_conn_latency, ping_latency) =
        match ping_diesel_db_mysql_async(context, duration).await {
            Ok((acquire_latency, ping_latency)) => (
                Status::Ok,
                Some(acquire_latency.as_millis()),
                Some(ping_latency.as_millis()),
            ),
            Err(err) => (
                Status::Err(ErrorData::builder().msg(err.to_string()).build()),
                None,
                None,
            ),
        };
    let db_timer = db_timer.elapsed();
    CheckResponse::builder()
        .status(db_status)
        .latency(db_timer)
        .custom(Latency {
            acquire_conn_latency,
            ping_latency,
        })
        .build()
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-mysql-pool-async")]
#[instrument(skip_all)]
async fn ping_diesel_db_mysql_async(
    context: &AppContext,
    duration: Option<Duration>,
) -> RoadsterResult<(Duration, Duration)> {
    use diesel_async::pooled_connection::PoolableConnection;

    let timer = Instant::now();
    let mut conn = if let Some(duration) = duration {
        timeout(duration, context.diesel_mysql_pool_async().get()).await??
    } else {
        context.diesel_mysql_pool_async().get().await?
    };
    let acquire_conn_latency = timer.elapsed();

    let timer = Instant::now();
    conn.ping(&diesel_async::pooled_connection::RecyclingMethod::Fast)
        .await?;
    let ping_latency = timer.elapsed();

    Ok((acquire_conn_latency, ping_latency))
}

#[cfg(feature = "email-smtp")]
pub(crate) async fn smtp_health(context: &AppContext, duration: Option<Duration>) -> CheckResponse {
    let timer = Instant::now();
    let status = match ping_smtp(context.smtp(), duration).await {
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
        Err(
            crate::error::other::OtherError::Message("Not connected to the SMTP server".to_owned())
                .into(),
        )
    }
}

#[cfg(feature = "worker-sidekiq")]
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

#[cfg(feature = "worker-sidekiq")]
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
        Err(crate::error::other::OtherError::Message(
            "Ping response does not match input.".to_owned(),
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use crate::health::check::{CheckResponse, ErrorData, MockHealthCheck, Status};
    use crate::testing::snapshot::TestCase;
    use insta::assert_json_snapshot;
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use std::time::Duration;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(Status::Ok, 100)]
    #[case(Status::Err(ErrorData::builder().msg("Error".to_string()).build()), 200)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn health_check_with_checks(
        _case: TestCase,
        #[case] status: Status,
        #[case] latency: u64,
    ) {
        // Arrange
        let mut check = MockHealthCheck::default();
        check.expect_name().return_const("example".to_string());
        check.expect_check().return_once(move || {
            Ok(CheckResponse::builder()
                .latency(Duration::from_millis(latency))
                .status(status)
                .build())
        });

        // Act
        let health_response = super::health_check_with_checks(vec![Arc::new(check)], None)
            .await
            .unwrap();

        // Assert
        assert_json_snapshot!(health_response, {".latency" => 1});
    }

    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn health_check_with_checks_error() {
        // Arrange
        let mut check = MockHealthCheck::default();
        check.expect_name().return_const("example".to_string());
        check.expect_check().return_once(move || {
            Err(crate::error::other::OtherError::Message("Error".to_owned()).into())
        });

        // Act
        let health_response = super::health_check_with_checks(vec![Arc::new(check)], None)
            .await
            .unwrap();

        // Assert
        assert_json_snapshot!(health_response, {".latency" => 1, ".resources.example.latency" => 1});
    }
}
