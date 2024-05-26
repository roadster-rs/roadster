use crate::app::App;
use crate::config::app_config::AppConfig;
use crate::error::RoadsterResult;
#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[cfg(not(test))]
type Inner = AppContextInner;
#[cfg(test)]
type Inner = MockAppContextInner;

#[derive(Clone)]
pub struct AppContext<T = ()> {
    inner: Arc<Inner>,
    custom: Arc<T>,
}

impl<T> AppContext<T> {
    // This method isn't used when running tests; only the mocked version is used.
    #[cfg_attr(test, allow(dead_code))]
    // The `A` type parameter isn't used in some feature configurations
    #[allow(clippy::extra_unused_type_parameters)]
    pub(crate) async fn new<A>(config: AppConfig) -> RoadsterResult<AppContext<()>>
    where
        A: App,
    {
        #[cfg(test)]
        // The `config.clone()` here is technically not necessary. However, without it, RustRover
        // is giving a "value used after move" error when creating an actual `AppContext` below.
        let context = { Self::test(Some(config.clone()), None)? };

        #[cfg(not(test))]
        let context = {
            #[cfg(feature = "db-sql")]
            let db = sea_orm::Database::connect(A::db_connection_options(&config)?).await?;

            #[cfg(feature = "sidekiq")]
            let (redis_enqueue, redis_fetch) = {
                let sidekiq_config = &config.service.sidekiq;
                let redis_config = &sidekiq_config.custom.redis;
                let redis = sidekiq::RedisConnectionManager::new(redis_config.uri.to_string())?;
                let redis_enqueue = {
                    let pool = bb8::Pool::builder().min_idle(redis_config.enqueue_pool.min_idle);
                    let pool = redis_config
                        .enqueue_pool
                        .max_connections
                        .iter()
                        .fold(pool, |pool, max_conns| pool.max_size(*max_conns));
                    pool.build(redis.clone()).await?
                };
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
                    Some(pool.build(redis.clone()).await?)
                };
                (redis_enqueue, redis_fetch)
            };
            let inner = AppContextInner {
                config,
                #[cfg(feature = "db-sql")]
                db,
                #[cfg(feature = "sidekiq")]
                redis_enqueue,
                #[cfg(feature = "sidekiq")]
                redis_fetch,
            };
            AppContext {
                inner: Arc::new(inner),
                custom: Arc::new(()),
            }
        };

        Ok(context)
    }

    #[cfg(test)]
    pub(crate) fn test(
        config: Option<AppConfig>,
        #[cfg(not(feature = "sidekiq"))] _redis: Option<()>,
        #[cfg(feature = "sidekiq")] redis: Option<sidekiq::RedisPool>,
    ) -> RoadsterResult<AppContext<()>> {
        let mut inner = MockAppContextInner::default();
        inner
            .expect_config()
            .return_const(config.unwrap_or(AppConfig::test(None)?));
        #[cfg(feature = "sidekiq")]
        if let Some(redis) = redis {
            inner.expect_redis_enqueue().return_const(redis.clone());
            inner.expect_redis_fetch().return_const(Some(redis));
        } else {
            inner.expect_redis_fetch().return_const(None);
        }
        Ok(AppContext {
            inner: Arc::new(inner),
            custom: Arc::new(()),
        })
    }

    pub fn with_custom<NewT: 'static>(self, custom: NewT) -> AppContext<NewT> {
        AppContext {
            inner: self.inner,
            custom: Arc::new(custom),
        }
    }

    pub fn config(&self) -> &AppConfig {
        self.inner.config()
    }

    #[cfg(feature = "db-sql")]
    pub fn db(&self) -> &DatabaseConnection {
        self.inner.db()
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_enqueue(&self) -> &sidekiq::RedisPool {
        self.inner.redis_enqueue()
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_fetch(&self) -> &Option<sidekiq::RedisPool> {
        self.inner.redis_fetch()
    }

    pub fn custom(&self) -> &T {
        &self.custom
    }
}

struct AppContextInner {
    config: AppConfig,
    #[cfg(feature = "db-sql")]
    db: DatabaseConnection,
    #[cfg(feature = "sidekiq")]
    redis_enqueue: sidekiq::RedisPool,
    /// The Redis connection pool used by [sidekiq::Processor] to fetch Sidekiq jobs from Redis.
    /// May be `None` if the [fetch_pool.max_connections][crate::config::service::worker::sidekiq::ConnectionPool]
    /// config is set to zero, in which case the [sidekiq::Processor] would also not be started.
    #[cfg(feature = "sidekiq")]
    redis_fetch: Option<sidekiq::RedisPool>,
}

#[cfg_attr(test, mockall::automock)]
#[cfg_attr(test, allow(dead_code))]
impl AppContextInner {
    fn config(&self) -> &AppConfig {
        &self.config
    }

    #[cfg(feature = "db-sql")]
    fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    #[cfg(feature = "sidekiq")]
    fn redis_enqueue(&self) -> &sidekiq::RedisPool {
        &self.redis_enqueue
    }

    #[cfg(feature = "sidekiq")]
    fn redis_fetch(&self) -> &Option<sidekiq::RedisPool> {
        &self.redis_fetch
    }
}
