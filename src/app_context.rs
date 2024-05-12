use std::sync::Arc;

#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;

use crate::config::app_config::AppConfig;

#[derive(Clone)]
pub struct AppContext<T = ()> {
    inner: Arc<AppContextInner>,
    custom: Arc<T>,
}

impl<T> AppContext<T> {
    pub(crate) fn new(
        config: AppConfig,
        #[cfg(feature = "db-sql")] db: DatabaseConnection,
        #[cfg(feature = "sidekiq")] redis_enqueue: sidekiq::RedisPool,
        #[cfg(feature = "sidekiq")] redis_fetch: Option<sidekiq::RedisPool>,
    ) -> anyhow::Result<AppContext<()>> {
        let inner = AppContextInner {
            config,
            #[cfg(feature = "db-sql")]
            db,
            #[cfg(feature = "sidekiq")]
            redis_enqueue,
            #[cfg(feature = "sidekiq")]
            redis_fetch,
        };
        Ok(AppContext {
            inner: Arc::new(inner),
            custom: Arc::new(()),
        })
    }

    pub fn with_custom<NewT>(self, custom: NewT) -> AppContext<NewT> {
        AppContext {
            inner: self.inner,
            custom: Arc::new(custom),
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.inner.config
    }

    #[cfg(feature = "db-sql")]
    pub fn db(&self) -> &DatabaseConnection {
        &self.inner.db
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_enqueue(&self) -> &sidekiq::RedisPool {
        &self.inner.redis_enqueue
    }

    #[cfg(feature = "sidekiq")]
    pub fn redis_fetch(&self) -> Option<&sidekiq::RedisPool> {
        self.inner.redis_fetch.as_ref()
    }

    pub fn custom(&self) -> &T {
        &self.custom
    }
}

#[derive(Debug)]
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

/// Implemented so consumers can use [AppContext] as their [crate::app::App::State] if they want.
impl From<Arc<AppContext>> for AppContext {
    fn from(value: Arc<AppContext>) -> Self {
        value.as_ref().clone()
    }
}
