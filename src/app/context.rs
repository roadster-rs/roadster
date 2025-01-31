use crate::app::metadata::AppMetadata;
use crate::app::App;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::health::check::HealthCheck;
use anyhow::anyhow;
use axum_core::extract::FromRef;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use std::sync::{Arc, OnceLock, Weak};

#[cfg(not(test))]
type Inner = AppContextInner;
#[cfg(test)]
type Inner = MockAppContextInner;

#[derive(Clone)]
pub struct AppContext {
    inner: Arc<Inner>,
}

/// A version of [`AppContext`] that holds a [`Weak`] pointer to the inner context. Useful for
/// preventing reference cycles between things that are held in the [`AppContext`] and also
/// need a reference to the [`AppContext`]; for example, [`HealthCheck`]s.
#[derive(Clone)]
pub struct AppContextWeak {
    inner: Weak<Inner>,
}

impl AppContextWeak {
    /// Get an [`AppContext`] from [`Self`].
    pub fn upgrade(&self) -> Option<AppContext> {
        self.inner.upgrade().map(|inner| AppContext { inner })
    }
}

impl AppContext {
    // This method isn't used when running tests; only the mocked version is used.
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) async fn new<A, S>(
        #[allow(unused_variables)] app: &A,
        #[cfg(not(feature = "test-containers"))] config: AppConfig,
        #[cfg(feature = "test-containers")]
        #[allow(unused_mut)]
        mut config: AppConfig,
        metadata: AppMetadata,
    ) -> RoadsterResult<Self>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        A: App<S>,
    {
        #[cfg(test)]
        // The `config.clone()` here is technically not necessary. However, without it, RustRover
        // is giving a "value used after move" error when creating an actual `AppContext` below.
        let context = { Self::test(Some(config.clone()), Some(metadata.clone()), None)? };

        #[cfg(not(test))]
        let context = {
            #[cfg(all(feature = "db-sql", feature = "test-containers"))]
            let db_test_container = db_test_container(&mut config).await?;
            #[cfg(all(feature = "sidekiq", feature = "test-containers"))]
            let sidekiq_redis_test_container = sidekiq_redis_test_container(&mut config).await?;

            #[cfg(feature = "db-sql")]
            let db = sea_orm::Database::connect(app.db_connection_options(&config)?).await?;

            #[cfg(feature = "sidekiq")]
            let (redis_enqueue, redis_fetch) = {
                let sidekiq_config = &config.service.sidekiq;
                let redis_config = &sidekiq_config.custom.redis;
                let redis = sidekiq::RedisConnectionManager::new(redis_config.uri.to_string())?;
                let redis_enqueue = RedisEnqueue::from({
                    let pool = bb8::Pool::builder().min_idle(redis_config.enqueue_pool.min_idle);
                    let pool = redis_config
                        .enqueue_pool
                        .max_connections
                        .iter()
                        .fold(pool, |pool, max_conns| pool.max_size(*max_conns));
                    pool.build(redis.clone()).await?
                });
                let redis_fetch = if redis_config
                    .fetch_pool
                    .max_connections
                    .iter()
                    .any(|max_conns| *max_conns == 0)
                {
                    tracing::info!("Redis fetch pool configured with size of zero, will not start the Sidekiq processor");
                    None
                } else {
                    let pool = bb8::Pool::builder().min_idle(redis_config.fetch_pool.min_idle);
                    let pool = redis_config
                        .fetch_pool
                        .max_connections
                        .iter()
                        .fold(pool, |pool, max_conns| pool.max_size(*max_conns));
                    Some(RedisFetch::from(pool.build(redis.clone()).await?))
                };
                (redis_enqueue, redis_fetch)
            };

            #[cfg(feature = "email-smtp")]
            let smtp = lettre::SmtpTransport::try_from(&config.email.smtp)?;

            #[cfg(feature = "email-sendgrid")]
            let sendgrid = sendgrid::v3::Sender::try_from(&config.email.sendgrid)?;

            let inner = AppContextInner {
                config,
                metadata,
                health_checks: OnceLock::new(),
                #[cfg(feature = "db-sql")]
                db,
                #[cfg(all(feature = "db-sql", feature = "test-containers"))]
                db_test_container,
                #[cfg(feature = "sidekiq")]
                redis_enqueue,
                #[cfg(feature = "sidekiq")]
                redis_fetch,
                #[cfg(all(feature = "sidekiq", feature = "test-containers"))]
                sidekiq_redis_test_container,
                #[cfg(feature = "email-smtp")]
                smtp,
                #[cfg(feature = "email-sendgrid")]
                sendgrid,
            };
            AppContext {
                inner: Arc::new(inner),
            }
        };

        Ok(context)
    }

    /// Get an [`AppContextWeak`] from [`Self`].
    pub fn downgrade(&self) -> AppContextWeak {
        AppContextWeak {
            inner: Arc::downgrade(&self.inner),
        }
    }

    #[cfg(test)]
    pub(crate) fn test(
        config: Option<AppConfig>,
        metadata: Option<AppMetadata>,
        #[cfg(not(feature = "sidekiq"))] _redis: Option<()>,
        #[cfg(feature = "sidekiq")] redis: Option<sidekiq::RedisPool>,
    ) -> RoadsterResult<Self> {
        let mut inner = MockAppContextInner::default();
        inner
            .expect_config()
            .return_const(config.unwrap_or(AppConfig::test(None)?));

        inner
            .expect_metadata()
            .return_const(metadata.unwrap_or_default());

        #[cfg(feature = "sidekiq")]
        if let Some(redis) = redis {
            inner
                .expect_redis_enqueue()
                .return_const(RedisEnqueue::from(redis.clone()));
            inner
                .expect_redis_fetch()
                .return_const(Some(RedisFetch::from(redis)));
        } else {
            inner.expect_redis_fetch().return_const(None);
        }
        Ok(AppContext {
            inner: Arc::new(inner),
        })
    }

    pub fn config(&self) -> &AppConfig {
        self.inner.config()
    }

    pub fn metadata(&self) -> &AppMetadata {
        self.inner.metadata()
    }

    /// Returns the [`HealthCheck`]s that were registered in the [`HealthCheckRegistry`], or
    /// an empty [`Vec`] if no [`HealthCheck`]s were registered.
    pub fn health_checks(&self) -> Vec<Arc<dyn HealthCheck>> {
        self.inner.health_checks()
    }

    pub(crate) fn set_health_checks(
        &self,
        health_checks: HealthCheckRegistry,
    ) -> RoadsterResult<()> {
        self.inner.set_health_checks(health_checks)
    }

    #[cfg(feature = "db-sql")]
    pub fn db(&self) -> &DatabaseConnection {
        self.inner.db()
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_enqueue(&self) -> &RedisEnqueue {
        self.inner.redis_enqueue()
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_fetch(&self) -> &Option<RedisFetch> {
        self.inner.redis_fetch()
    }

    #[cfg(feature = "email-smtp")]
    pub fn mailer(&self) -> &lettre::SmtpTransport {
        self.inner.smtp()
    }

    #[cfg(feature = "email-smtp")]
    pub fn smtp(&self) -> &lettre::SmtpTransport {
        self.inner.smtp()
    }

    #[cfg(feature = "email-sendgrid")]
    pub fn sendgrid(&self) -> &sendgrid::v3::Sender {
        self.inner.sendgrid()
    }
}

/// Trait to allow getting a reference to the `T` from the implementing type. [`AppContext`]
/// implements this for various types it contains. This allows a method to specify the type it
/// requires, then the caller of the method can determine how to provide the type. This is a
/// similar concept to dependency injection (DI) in frameworks like Java Spring, though this
/// is far from a full DI system.
///
/// This is useful, for example, to allow mocking the DB connection in tests. Your DB operation
/// method would declare a parameter of type `ProvideRef<DataBaseConnection>`, then your application
/// code would provide the [`AppContext`] to the method, and your tests could provide a mocked
/// [`ProvideRef`] instance that returns a mock DB connection. Note that mocking the DB comes with
/// its own set of trade-offs, for example, it may not exactly match the behavior of an actual DB
/// that's used in production. Consider testing against an actual DB instead of mocking, e.g.,
/// by using test containers.
///
/// A mocked implementation of the trait is provided if the `testing-mocks` feature is enabled.
///
/// See also:
/// - [SeaORM Mock Interface](https://www.sea-ql.org/SeaORM/docs/write-test/mock/)
/// - [Test Containers](https://testcontainers.com/)
/// - [Roadster Testing docs](https://roadster.dev/features/testing.html/)
// Todo: add code example
#[cfg_attr(any(test, feature = "testing-mocks"), mockall::automock)]
pub trait ProvideRef<T> {
    fn provide(&self) -> &T;
}

/// Trait to allow getting an instance of `T` from the implementing type. [`AppContext`]
/// implements this for various types it contains. This allows a method to specify the type it
/// requires, then the caller of the method can determine how to provide the type. This is a
/// similar concept to dependency injection (DI) in frameworks like Java Spring, though this
/// is far from a full DI system.
///
/// This is useful, for example, to allow mocking the DB connection in tests. Your DB operation
/// method would declare a parameter of type `Provide<DataBaseConnection>`, then your application
/// code would provide the [`AppContext`] to the method, and your tests could provide a mocked
/// [`Provide`] instance that returns a mock DB connection. Note that mocking the DB comes with
/// its own set of trade-offs, for example, it may not exactly match the behavior of an actual DB
/// that's used in production. Consider testing against an actual DB instead of mocking, e.g.,
/// by using test containers.
///
/// A mocked implementation of the trait is provided if the `testing-mocks` feature is enabled.
///
/// See also:
/// - [SeaORM Mock Interface](https://www.sea-ql.org/SeaORM/docs/write-test/mock/)
/// - [Test Containers](https://testcontainers.com/)
/// - [Roadster Testing docs](https://roadster.dev/features/testing.html/)
// Todo: add code example
#[cfg_attr(any(test, feature = "testing-mocks"), mockall::automock)]
pub trait Provide<T> {
    fn provide(&self) -> T;
}

impl ProvideRef<AppConfig> for AppContext {
    fn provide(&self) -> &AppConfig {
        self.config()
    }
}

impl Provide<AppConfig> for AppContext {
    fn provide(&self) -> AppConfig {
        self.config().clone()
    }
}

impl ProvideRef<AppMetadata> for AppContext {
    fn provide(&self) -> &AppMetadata {
        self.metadata()
    }
}

impl Provide<AppMetadata> for AppContext {
    fn provide(&self) -> AppMetadata {
        self.metadata().clone()
    }
}

impl Provide<Vec<Arc<dyn HealthCheck>>> for AppContext {
    fn provide(&self) -> Vec<Arc<dyn HealthCheck>> {
        self.health_checks()
    }
}

#[cfg(feature = "db-sql")]
impl ProvideRef<DatabaseConnection> for AppContext {
    fn provide(&self) -> &DatabaseConnection {
        self.db()
    }
}

/// Unfortunately, [`Provide<DatabaseConnection>`] can not be implemented when the `sea-orm/mock`
/// feature is enabled because `MockDatabase` is not [`Clone`]
#[cfg(all(feature = "db-sql", not(feature = "testing-mocks")))]
impl Provide<DatabaseConnection> for AppContext {
    fn provide(&self) -> DatabaseConnection {
        self.db().clone()
    }
}

#[cfg(feature = "email-smtp")]
impl ProvideRef<lettre::SmtpTransport> for AppContext {
    fn provide(&self) -> &lettre::SmtpTransport {
        self.smtp()
    }
}

#[cfg(feature = "email-smtp")]
impl Provide<lettre::SmtpTransport> for AppContext {
    fn provide(&self) -> lettre::SmtpTransport {
        self.smtp().clone()
    }
}

#[cfg(feature = "email-sendgrid")]
impl ProvideRef<sendgrid::v3::Sender> for AppContext {
    fn provide(&self) -> &sendgrid::v3::Sender {
        self.sendgrid()
    }
}

#[cfg(feature = "email-sendgrid")]
impl Provide<sendgrid::v3::Sender> for AppContext {
    fn provide(&self) -> sendgrid::v3::Sender {
        self.sendgrid().clone()
    }
}

#[cfg(feature = "sidekiq")]
#[derive(Clone)]
#[non_exhaustive]
pub struct RedisEnqueue {
    pub inner: sidekiq::RedisPool,
}

#[cfg(feature = "sidekiq")]
impl From<sidekiq::RedisPool> for RedisEnqueue {
    fn from(value: sidekiq::RedisPool) -> Self {
        Self { inner: value }
    }
}

#[cfg(feature = "sidekiq")]
#[derive(Clone)]
#[non_exhaustive]
pub struct RedisFetch {
    pub inner: sidekiq::RedisPool,
}

#[cfg(feature = "sidekiq")]
impl From<sidekiq::RedisPool> for RedisFetch {
    fn from(value: sidekiq::RedisPool) -> Self {
        Self { inner: value }
    }
}

#[cfg(feature = "sidekiq")]
impl std::ops::Deref for RedisEnqueue {
    type Target = sidekiq::RedisPool;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "sidekiq")]
impl std::ops::Deref for RedisFetch {
    type Target = sidekiq::RedisPool;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(feature = "sidekiq")]
impl Provide<RedisEnqueue> for AppContext {
    fn provide(&self) -> RedisEnqueue {
        self.redis_enqueue().clone()
    }
}

#[cfg(feature = "sidekiq")]
impl ProvideRef<RedisEnqueue> for AppContext {
    fn provide(&self) -> &RedisEnqueue {
        self.inner.redis_enqueue()
    }
}

#[cfg(feature = "sidekiq")]
impl Provide<Option<RedisFetch>> for AppContext {
    fn provide(&self) -> Option<RedisFetch> {
        self.redis_fetch().as_ref().cloned()
    }
}

#[cfg(all(feature = "db-sql", feature = "test-containers"))]
#[cfg_attr(test, allow(dead_code))]
async fn db_test_container(
    config: &mut AppConfig,
) -> RoadsterResult<
    Option<
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::postgres::Postgres,
        >,
    >,
> {
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use testcontainers_modules::testcontainers::ImageExt;

    let container = if let Some(test_container) = config.database.test_container.as_ref() {
        let container = testcontainers_modules::postgres::Postgres::default()
            .with_tag(test_container.tag.to_string())
            .start()
            .await
            .map_err(|err| anyhow!("{err}"))?;
        Some(container)
    } else {
        None
    };

    if let Some(container) = container.as_ref() {
        let host_ip = container.get_host().await.map_err(|err| anyhow!("{err}"))?;

        let host_port = container
            .get_host_port_ipv4(5432)
            .await
            .map_err(|err| anyhow!("{err}"))?;

        config.database.uri =
            format!("postgres://postgres:postgres@{host_ip}:{host_port}/postgres").parse()?;
    }
    Ok(container)
}

#[cfg(all(feature = "sidekiq", feature = "test-containers"))]
#[cfg_attr(test, allow(dead_code))]
async fn sidekiq_redis_test_container(
    config: &mut AppConfig,
) -> RoadsterResult<
    Option<
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::redis::Redis,
        >,
    >,
> {
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use testcontainers_modules::testcontainers::ImageExt;

    let container =
        if let Some(test_container) = config.service.sidekiq.custom.redis.test_container.as_ref() {
            let container = testcontainers_modules::redis::Redis::default()
                .with_tag(test_container.tag.to_string())
                .start()
                .await
                .map_err(|err| anyhow!("{err}"))?;
            Some(container)
        } else {
            None
        };

    if let Some(container) = container.as_ref() {
        let host_ip = container.get_host().await.map_err(|err| anyhow!("{err}"))?;

        let host_port = container
            .get_host_port_ipv4(testcontainers_modules::redis::REDIS_PORT)
            .await
            .map_err(|err| anyhow!("{err}"))?;

        config.service.sidekiq.custom.redis.uri =
            format!("redis://{host_ip}:{host_port}").parse()?;
    }
    Ok(container)
}

struct AppContextInner {
    config: AppConfig,
    metadata: AppMetadata,
    health_checks: OnceLock<HealthCheckRegistry>,
    #[cfg(feature = "db-sql")]
    db: DatabaseConnection,
    #[cfg(all(feature = "db-sql", feature = "test-containers"))]
    #[allow(dead_code)]
    db_test_container: Option<
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::postgres::Postgres,
        >,
    >,
    #[cfg(feature = "sidekiq")]
    redis_enqueue: RedisEnqueue,
    /// The Redis connection pool used by [sidekiq::Processor] to fetch Sidekiq jobs from Redis.
    /// May be `None` if the [fetch_pool.max_connections][crate::config::service::worker::sidekiq::ConnectionPool]
    /// config is set to zero, in which case the [sidekiq::Processor] would also not be started.
    #[cfg(feature = "sidekiq")]
    redis_fetch: Option<RedisFetch>,
    #[cfg(all(feature = "sidekiq", feature = "test-containers"))]
    #[allow(dead_code)]
    sidekiq_redis_test_container: Option<
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::redis::Redis,
        >,
    >,
    #[cfg(feature = "email-smtp")]
    smtp: lettre::SmtpTransport,
    #[cfg(feature = "email-sendgrid")]
    sendgrid: sendgrid::v3::Sender,
}

#[cfg_attr(test, mockall::automock)]
#[cfg_attr(test, allow(dead_code))]
impl AppContextInner {
    fn config(&self) -> &AppConfig {
        &self.config
    }

    fn metadata(&self) -> &AppMetadata {
        &self.metadata
    }

    fn health_checks(&self) -> Vec<Arc<dyn HealthCheck>> {
        self.health_checks
            .get()
            .map(|health_checks| health_checks.checks())
            .unwrap_or_default()
    }

    fn set_health_checks(&self, health_checks: HealthCheckRegistry) -> RoadsterResult<()> {
        self.health_checks
            .set(health_checks)
            .map_err(|_| anyhow!("Unable to set health check registry"))?;

        Ok(())
    }

    #[cfg(feature = "db-sql")]
    fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    #[cfg(feature = "sidekiq")]
    fn redis_enqueue(&self) -> &RedisEnqueue {
        &self.redis_enqueue
    }

    #[cfg(feature = "sidekiq")]
    fn redis_fetch(&self) -> &Option<RedisFetch> {
        &self.redis_fetch
    }

    #[cfg(feature = "email-smtp")]
    fn smtp(&self) -> &lettre::SmtpTransport {
        &self.smtp
    }

    #[cfg(feature = "email-sendgrid")]
    fn sendgrid(&self) -> &sendgrid::v3::Sender {
        &self.sendgrid
    }
}
