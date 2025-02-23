use crate::app::metadata::AppMetadata;
use crate::app::App;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::health::check::registry::HealthCheckRegistry;
use crate::health::check::HealthCheck;
use anyhow::anyhow;
use axum_core::extract::FromRef;
#[cfg(feature = "db-sea-orm")]
use sea_orm::DatabaseConnection;
use std::sync::{Arc, OnceLock, Weak};

#[cfg(feature = "db-diesel-postgres")]
pub type DieselPgConn = diesel::pg::PgConnection;
#[cfg(feature = "db-diesel-mysql")]
pub type DieselMysqlConn = diesel::mysql::MysqlConnection;
#[cfg(feature = "db-diesel-sqlite")]
pub type DieselSqliteConn = diesel::sqlite::SqliteConnection;

#[cfg(feature = "db-diesel-postgres-pool")]
pub type DieselPgPool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselPgConn>>;
#[cfg(feature = "db-diesel-mysql-pool")]
pub type DieselMysqlPool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselMysqlConn>>;
#[cfg(feature = "db-diesel-sqlite-pool")]
pub type DieselSqlitePool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselSqliteConn>>;

#[cfg(feature = "db-diesel-postgres-pool-async")]
pub type DieselPgConnAsync = diesel_async::AsyncPgConnection;
#[cfg(feature = "db-diesel-mysql-pool-async")]
pub type DieselMysqlConnAsync = diesel_async::AsyncMysqlConnection;

#[cfg(feature = "db-diesel-postgres-pool-async")]
pub type DieselPgPoolAsync = diesel_async::pooled_connection::bb8::Pool<DieselPgConnAsync>;
#[cfg(feature = "db-diesel-mysql-pool-async")]
pub type DieselMysqlPoolAsync = diesel_async::pooled_connection::bb8::Pool<DieselMysqlConnAsync>;

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
        #[cfg(not(feature = "testing"))] config: AppConfig,
        #[cfg(feature = "testing")]
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

            #[cfg(all(feature = "db-sql", feature = "testing"))]
            let temporary_test_db = create_temporary_test_db(&mut config).await?;

            #[cfg(feature = "db-sea-orm")]
            let sea_orm =
                sea_orm::Database::connect(app.sea_orm_connection_options(&config)?).await?;

            #[cfg(feature = "db-diesel-postgres-pool")]
            let diesel_pg_pool = build_diesel_pool::<DieselPgConn>(&config)?;

            #[cfg(feature = "db-diesel-mysql-pool")]
            let diesel_mysql_pool = build_diesel_pool::<DieselMysqlConn>(&config)?;

            #[cfg(feature = "db-diesel-sqlite-pool")]
            let diesel_sqlite_pool = build_diesel_pool::<DieselSqliteConn>(&config)?;

            #[cfg(feature = "db-diesel-postgres-pool-async")]
            let diesel_pg_pool_async = build_diesel_pg_async_pool(&config).await?;

            #[cfg(feature = "db-diesel-mysql-pool-async")]
            let diesel_mysql_pool_async = build_diesel_mysql_async_pool(&config).await?;

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
                #[cfg(feature = "db-sea-orm")]
                sea_orm,
                #[cfg(feature = "db-diesel-postgres-pool")]
                diesel_pg_pool,
                #[cfg(feature = "db-diesel-mysql-pool")]
                diesel_mysql_pool,
                #[cfg(feature = "db-diesel-sqlite-pool")]
                diesel_sqlite_pool,
                #[cfg(feature = "db-diesel-postgres-pool-async")]
                diesel_pg_pool_async,
                #[cfg(feature = "db-diesel-mysql-pool-async")]
                diesel_mysql_pool_async,
                #[cfg(all(feature = "db-sql", feature = "test-containers"))]
                db_test_container,
                #[cfg(all(feature = "db-sql", feature = "testing"))]
                temporary_test_db,
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

    #[cfg(feature = "testing")]
    pub(crate) async fn teardown(&self) -> RoadsterResult<()> {
        self.inner.teardown().await
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

    /// Get the app's [`AppContext`].
    pub fn config(&self) -> &AppConfig {
        self.inner.config()
    }

    /// Get the app's [`AppMetadata`]
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

    /// Get the sea-orm DB connection pool.
    #[cfg(feature = "db-sea-orm")]
    pub fn sea_orm(&self) -> &DatabaseConnection {
        self.inner.sea_orm()
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    pub fn diesel_pg_pool(&self) -> &DieselPgPool {
        self.inner.diesel_pg_pool()
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    pub fn diesel_mysql_pool(&self) -> &DieselMysqlPool {
        self.inner.diesel_mysql_pool()
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    pub fn diesel_sqlite_pool(&self) -> &DieselSqlitePool {
        self.inner.diesel_sqlite_pool()
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    pub fn diesel_pg_pool_async(&self) -> &DieselPgPoolAsync {
        self.inner.diesel_pg_pool_async()
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    pub fn diesel_mysql_pool_async(&self) -> &DieselMysqlPoolAsync {
        self.inner.diesel_mysql_pool_async()
    }

    /// Get the Redis connection pool used to enqueue Sidekiq jobs.
    #[cfg(feature = "sidekiq")]
    pub fn redis_enqueue(&self) -> &RedisEnqueue {
        self.inner.redis_enqueue()
    }

    /// Get the Redis connection pool used to fetch Sidekiq jobs. This shouldn't be needed by most
    /// applications but is provided as a convenience in case it is.
    #[cfg(feature = "sidekiq")]
    pub fn redis_fetch(&self) -> &Option<RedisFetch> {
        self.inner.redis_fetch()
    }

    /// Get the SMTP client. Used to send emails via the SMTP protocol
    #[cfg(feature = "email-smtp")]
    pub fn smtp(&self) -> &lettre::SmtpTransport {
        self.inner.smtp()
    }

    /// Get the Sendgrid client. Used to send emails via Sendgrid's Mail Send API.
    #[cfg(feature = "email-sendgrid")]
    pub fn sendgrid(&self) -> &sendgrid::v3::Sender {
        self.inner.sendgrid()
    }
}

#[cfg(any(
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool",
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async"
))]
#[derive(Debug)]
#[cfg_attr(test, allow(dead_code))]
struct TracingErrorHandler;

#[cfg(any(
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool",
))]
#[cfg_attr(test, allow(dead_code))]
impl<E> r2d2::HandleError<E> for TracingErrorHandler
where
    E: std::error::Error,
{
    fn handle_error(&self, err: E) {
        tracing::error!("DB connection pool error: {err}");
    }
}

#[cfg(any(
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async"
))]
#[cfg_attr(test, allow(dead_code))]
impl bb8_8::ErrorSink<diesel_async::pooled_connection::PoolError> for TracingErrorHandler {
    fn sink(&self, err: diesel_async::pooled_connection::PoolError) {
        tracing::error!("DB connection pool error: {err}");
    }

    fn boxed_clone(&self) -> Box<dyn bb8_8::ErrorSink<diesel_async::pooled_connection::PoolError>> {
        Box::new(TracingErrorHandler)
    }
}

#[cfg(any(
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool",
))]
#[cfg_attr(test, allow(dead_code))]
fn build_diesel_pool<C>(
    config: &AppConfig,
) -> RoadsterResult<r2d2::Pool<diesel::r2d2::ConnectionManager<C>>>
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
{
    let url = config.database.uri.clone();
    let manager: diesel::r2d2::ConnectionManager<C> = diesel::r2d2::ConnectionManager::new(url);

    let builder = r2d2::Pool::builder()
        .error_handler(Box::new(TracingErrorHandler))
        .test_on_check_out(config.database.test_on_checkout)
        .min_idle(Some(config.database.min_connections))
        .max_size(config.database.max_connections)
        .idle_timeout(config.database.idle_timeout)
        .connection_timeout(config.database.connect_timeout)
        .max_lifetime(config.database.max_lifetime);
    let pool = if config.database.connect_lazy {
        builder.build_unchecked(manager)
    } else {
        builder.build(manager)?
    };

    Ok(pool)
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-postgres-pool-async")]
#[cfg_attr(test, allow(dead_code))]
async fn build_diesel_pg_async_pool(config: &AppConfig) -> RoadsterResult<DieselPgPoolAsync> {
    let url = config.database.uri.clone();
    let manager =
        diesel_async::pooled_connection::AsyncDieselConnectionManager::<DieselPgConnAsync>::new(
            url,
        );

    let builder = diesel_async::pooled_connection::bb8::Pool::builder()
        .error_sink(Box::new(TracingErrorHandler))
        .test_on_check_out(config.database.test_on_checkout)
        .min_idle(Some(config.database.min_connections))
        .max_size(config.database.max_connections)
        .idle_timeout(config.database.idle_timeout)
        .connection_timeout(config.database.connect_timeout)
        .retry_connection(config.database.retry_connection)
        .max_lifetime(config.database.max_lifetime);
    let pool = if config.database.connect_lazy {
        builder.build_unchecked(manager)
    } else {
        builder.build(manager).await?
    };

    Ok(pool)
}

// Todo: reduce duplication
#[cfg(feature = "db-diesel-mysql-pool-async")]
#[cfg_attr(test, allow(dead_code))]
async fn build_diesel_mysql_async_pool(config: &AppConfig) -> RoadsterResult<DieselMysqlPoolAsync> {
    let url = config.database.uri.clone();
    let manager = diesel_async::pooled_connection::AsyncDieselConnectionManager::<
        DieselMysqlConnAsync,
    >::new(url);

    let builder = diesel_async::pooled_connection::bb8::Pool::builder()
        .error_sink(Box::new(TracingErrorHandler))
        .test_on_check_out(config.database.test_on_checkout)
        .min_idle(Some(config.database.min_connections))
        .max_size(config.database.max_connections)
        .idle_timeout(config.database.idle_timeout)
        .connection_timeout(config.database.connect_timeout)
        .retry_connection(config.database.retry_connection)
        .max_lifetime(config.database.max_lifetime);
    let pool = if config.database.connect_lazy {
        builder.build_unchecked(manager)
    } else {
        builder.build(manager).await?
    };

    Ok(pool)
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

#[cfg(feature = "db-sea-orm")]
impl ProvideRef<DatabaseConnection> for AppContext {
    fn provide(&self) -> &DatabaseConnection {
        self.sea_orm()
    }
}

/// Unfortunately, [`Provide<DatabaseConnection>`] can not be implemented when the `sea-orm/mock`
/// feature is enabled because `MockDatabase` is not [`Clone`]
#[cfg(all(feature = "db-sea-orm", not(feature = "testing-mocks")))]
impl Provide<DatabaseConnection> for AppContext {
    fn provide(&self) -> DatabaseConnection {
        self.sea_orm().clone()
    }
}

#[cfg(feature = "db-diesel-postgres-pool")]
impl ProvideRef<DieselPgPool> for AppContext {
    fn provide(&self) -> &DieselPgPool {
        self.diesel_pg_pool()
    }
}

#[cfg(feature = "db-diesel-mysql-pool")]
impl ProvideRef<DieselMysqlPool> for AppContext {
    fn provide(&self) -> &DieselMysqlPool {
        self.diesel_mysql_pool()
    }
}

#[cfg(feature = "db-diesel-sqlite-pool")]
impl ProvideRef<DieselSqlitePool> for AppContext {
    fn provide(&self) -> &DieselSqlitePool {
        self.diesel_sqlite_pool()
    }
}

#[cfg(feature = "db-diesel-postgres-pool-async")]
impl ProvideRef<DieselPgPoolAsync> for AppContext {
    fn provide(&self) -> &DieselPgPoolAsync {
        self.diesel_pg_pool_async()
    }
}

#[cfg(feature = "db-diesel-mysql-pool-async")]
impl ProvideRef<DieselMysqlPoolAsync> for AppContext {
    fn provide(&self) -> &DieselMysqlPoolAsync {
        self.diesel_mysql_pool_async()
    }
}

#[cfg(feature = "db-diesel-postgres-pool")]
impl Provide<DieselPgPool> for AppContext {
    fn provide(&self) -> DieselPgPool {
        self.diesel_pg_pool().clone()
    }
}

#[cfg(feature = "db-diesel-mysql-pool")]
impl Provide<DieselMysqlPool> for AppContext {
    fn provide(&self) -> DieselMysqlPool {
        self.diesel_mysql_pool().clone()
    }
}

#[cfg(feature = "db-diesel-sqlite-pool")]
impl Provide<DieselSqlitePool> for AppContext {
    fn provide(&self) -> DieselSqlitePool {
        self.diesel_sqlite_pool().clone()
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
enum DbTestContainer {
    Postgres(
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::postgres::Postgres,
        >,
    ),
    Mysql(
        testcontainers_modules::testcontainers::ContainerAsync<
            testcontainers_modules::mysql::Mysql,
        >,
    ),
}

#[cfg(all(feature = "db-sql", feature = "test-containers"))]
impl DbTestContainer {
    async fn get_host(
        &self,
    ) -> testcontainers_modules::testcontainers::core::error::Result<url::Host> {
        match self {
            DbTestContainer::Postgres(container) => container.get_host().await,
            DbTestContainer::Mysql(container) => container.get_host().await,
        }
    }

    async fn get_port(&self) -> testcontainers_modules::testcontainers::core::error::Result<u16> {
        match self {
            DbTestContainer::Postgres(container) => container.get_host_port_ipv4(5432).await,
            DbTestContainer::Mysql(container) => container.get_host_port_ipv4(3306).await,
        }
    }

    async fn get_uri(&self) -> RoadsterResult<url::Url> {
        let host = self.get_host().await.map_err(|err| anyhow!("{err}"))?;
        let port = self.get_port().await.map_err(|err| anyhow!("{err}"))?;
        let uri = match self {
            DbTestContainer::Postgres(_) => {
                format!("postgres://postgres:postgres@{host}:{port}/postgres").parse()?
            }
            DbTestContainer::Mysql(_) => format!("mysql://{host}:{port}/test").parse()?,
        };
        Ok(uri)
    }
}

#[cfg(all(feature = "db-sql", feature = "test-containers"))]
#[cfg_attr(test, allow(dead_code))]
async fn db_test_container(config: &mut AppConfig) -> RoadsterResult<Option<DbTestContainer>> {
    use testcontainers_modules::testcontainers::runners::AsyncRunner;
    use testcontainers_modules::testcontainers::ImageExt;

    let uri_scheme = config.database.uri.scheme();

    let container: Option<DbTestContainer> =
        if let Some(test_container) = config.database.test_container.as_ref() {
            if uri_scheme == "postgres" {
                let container = testcontainers_modules::postgres::Postgres::default()
                    .with_tag(test_container.tag.to_string())
                    .start()
                    .await
                    .map_err(|err| anyhow!("{err}"))?;
                Some(DbTestContainer::Postgres(container))
            } else if uri_scheme == "mysql" {
                let container = testcontainers_modules::mysql::Mysql::default()
                    .with_tag(test_container.tag.to_string())
                    .start()
                    .await
                    .map_err(|err| anyhow!("{err}"))?;
                Some(DbTestContainer::Mysql(container))
            } else {
                None
            }
        } else {
            None
        };

    if let Some(container) = container.as_ref() {
        config.database.uri = container.get_uri().await?;
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

#[cfg(all(feature = "db-sql", feature = "testing"))]
#[cfg_attr(test, allow(dead_code))]
async fn create_temporary_test_db(
    config: &mut AppConfig,
) -> RoadsterResult<Option<TemporaryTestDb>> {
    if !config.database.temporary_test_db {
        return Ok(None);
    }

    let original_uri = config.database.uri.clone();
    let db_name = uuid::Uuid::new_v4().to_string();
    tracing::debug!("Creating test db {db_name} using connection {original_uri}");

    #[allow(unused_variables)]
    let done = false;

    #[cfg(feature = "db-diesel")]
    let done = {
        crate::util::db::diesel::create_database(&original_uri, &db_name)?;
        true
    };

    #[cfg(feature = "db-sea-orm")]
    let done = {
        if !done {
            crate::util::db::sea_orm::create_database(&original_uri, &db_name).await?;
        }
        true
    };

    if done {
        let mut new_uri = original_uri.clone();
        new_uri.set_path(&db_name);
        config.database.uri = new_uri.clone();
    }

    if done && config.database.temporary_test_db_clean_up {
        Ok(Some(TemporaryTestDb {
            original_uri,
            db_name,
        }))
    } else {
        Ok(None)
    }
}

struct AppContextInner {
    config: AppConfig,
    metadata: AppMetadata,
    health_checks: OnceLock<HealthCheckRegistry>,
    #[cfg(feature = "db-sea-orm")]
    sea_orm: DatabaseConnection,
    #[cfg(feature = "db-diesel-postgres-pool")]
    diesel_pg_pool: DieselPgPool,
    #[cfg(feature = "db-diesel-mysql-pool")]
    diesel_mysql_pool: DieselMysqlPool,
    #[cfg(feature = "db-diesel-sqlite-pool")]
    diesel_sqlite_pool: DieselSqlitePool,
    #[cfg(feature = "db-diesel-postgres-pool-async")]
    diesel_pg_pool_async: DieselPgPoolAsync,
    #[cfg(feature = "db-diesel-mysql-pool-async")]
    diesel_mysql_pool_async: DieselMysqlPoolAsync,
    #[cfg(all(feature = "db-sql", feature = "test-containers"))]
    #[allow(dead_code)]
    db_test_container: Option<DbTestContainer>,
    #[cfg(all(feature = "db-sql", feature = "testing"))]
    temporary_test_db: Option<TemporaryTestDb>,
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
    #[cfg(feature = "testing")]
    async fn teardown(&self) -> RoadsterResult<()> {
        #[cfg(feature = "db-sql")]
        if let Some(temporary_test_db) = self.temporary_test_db.as_ref() {
            temporary_test_db.drop_temporary_test_db().await?;
        }

        Ok(())
    }

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

    #[cfg(feature = "db-sea-orm")]
    fn sea_orm(&self) -> &DatabaseConnection {
        &self.sea_orm
    }

    #[cfg(feature = "db-diesel-postgres-pool")]
    fn diesel_pg_pool(&self) -> &DieselPgPool {
        &self.diesel_pg_pool
    }

    #[cfg(feature = "db-diesel-mysql-pool")]
    fn diesel_mysql_pool(&self) -> &DieselMysqlPool {
        &self.diesel_mysql_pool
    }

    #[cfg(feature = "db-diesel-sqlite-pool")]
    fn diesel_sqlite_pool(&self) -> &DieselSqlitePool {
        &self.diesel_sqlite_pool
    }

    #[cfg(feature = "db-diesel-postgres-pool-async")]
    fn diesel_pg_pool_async(&self) -> &DieselPgPoolAsync {
        &self.diesel_pg_pool_async
    }

    #[cfg(feature = "db-diesel-mysql-pool-async")]
    fn diesel_mysql_pool_async(&self) -> &DieselMysqlPoolAsync {
        &self.diesel_mysql_pool_async
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

#[cfg(all(feature = "db-sql", feature = "testing"))]
struct TemporaryTestDb {
    original_uri: url::Url,
    db_name: String,
}

#[cfg(all(feature = "db-sql", feature = "testing"))]
impl TemporaryTestDb {
    #[cfg_attr(test, allow(dead_code))]
    async fn drop_temporary_test_db(&self) -> RoadsterResult<()> {
        #[allow(unused_variables)]
        let done = false;

        #[cfg(feature = "db-diesel")]
        let done = {
            crate::util::db::diesel::drop_database(&self.original_uri, &self.db_name).await?;
            true
        };

        #[cfg(feature = "db-sea-orm")]
        {
            if done {
                return Ok(());
            }
            crate::util::db::sea_orm::drop_database(&self.original_uri, &self.db_name).await?;
        };
        Ok(())
    }
}
