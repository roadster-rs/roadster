use std::sync::Arc;

#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;

use crate::config::app_config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppContext {
    inner: Arc<AppContextInner>,
}

impl AppContext {
    pub async fn new(
        config: AppConfig,
        #[cfg(feature = "db-sql")] db: DatabaseConnection,
        #[cfg(feature = "sidekiq")] redis_enqueue: sidekiq::RedisPool,
        #[cfg(feature = "sidekiq")] redis_fetch: Option<sidekiq::RedisPool>,
    ) -> anyhow::Result<Self> {
        let inner = AppContextInner {
            config,
            #[cfg(feature = "db-sql")]
            db,
            #[cfg(feature = "sidekiq")]
            redis_enqueue,
            #[cfg(feature = "sidekiq")]
            redis_fetch,
        };
        Ok(Self {
            inner: Arc::new(inner),
        })
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
