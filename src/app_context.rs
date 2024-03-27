#[cfg(feature = "open-api")]
use std::sync::Arc;

#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
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
    pub redis: sidekiq::RedisPool,
    #[cfg(feature = "open-api")]
    pub api: Arc<OpenApi>,
    // Prevent consumers from directly creating an AppContext
    _private: (),
}

impl AppContext {
    pub async fn new(
        config: AppConfig,
        #[cfg(feature = "db-sql")] db: DatabaseConnection,
        #[cfg(feature = "sidekiq")] redis: sidekiq::RedisPool,
        #[cfg(feature = "open-api")] api: Arc<OpenApi>,
    ) -> anyhow::Result<Self> {
        let context = Self {
            config,
            #[cfg(feature = "db-sql")]
            db,
            #[cfg(feature = "sidekiq")]
            redis,
            #[cfg(feature = "open-api")]
            api,
            _private: (),
        };
        Ok(context)
    }
}
