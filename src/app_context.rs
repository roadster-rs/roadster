use std::sync::Arc;

#[cfg(feature = "db-sql")]
use sea_orm::DatabaseConnection;

use crate::config::app_config::AppConfig;

#[derive(Debug, Clone)]
#[allow(clippy::manual_non_exhaustive)]
pub struct AppContext {
    pub config: AppConfig,
    #[cfg(feature = "db-sql")]
    pub db: DatabaseConnection,
    #[cfg(feature = "sidekiq")]
    pub redis_enqueue: sidekiq::RedisPool,
    /// The Redis connection pool used by [sidekiq::Processor] to fetch Sidekiq jobs from Redis.
    /// May be `None` if the [fetch_pool.max_connections][crate::config::service::worker::sidekiq::ConnectionPool]
    /// config is set to zero, in which case the [sidekiq::Processor] would also not be started.
    #[cfg(feature = "sidekiq")]
    pub redis_fetch: Option<sidekiq::RedisPool>,
    // Prevent consumers from directly creating an AppContext
    _private: (),
}

impl AppContext {
    pub async fn new(
        config: AppConfig,
        #[cfg(feature = "db-sql")] db: DatabaseConnection,
        #[cfg(feature = "sidekiq")] redis_enqueue: sidekiq::RedisPool,
        #[cfg(feature = "sidekiq")] redis_fetch: Option<sidekiq::RedisPool>,
    ) -> anyhow::Result<Self> {
        let context = Self {
            config,
            #[cfg(feature = "db-sql")]
            db,
            #[cfg(feature = "sidekiq")]
            redis_enqueue,
            #[cfg(feature = "sidekiq")]
            redis_fetch,
            _private: (),
        };
        Ok(context)
    }
}

/// Implemented so consumers can use [AppContext] as their [crate::app::App::State] if they want.
impl From<Arc<AppContext>> for AppContext {
    fn from(value: Arc<AppContext>) -> Self {
        value.as_ref().clone()
    }
}
