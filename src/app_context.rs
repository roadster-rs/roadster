use std::sync::Arc;

use aide::openapi::OpenApi;
use sea_orm::DatabaseConnection;

use crate::config::app_config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub db: DatabaseConnection,
    pub redis: Option<sidekiq::RedisPool>,
    pub api: Option<Arc<OpenApi>>,
}

impl AppContext {
    pub async fn new(
        config: AppConfig,
        db: DatabaseConnection,
        redis: Option<sidekiq::RedisPool>,
    ) -> anyhow::Result<Self> {
        let context = Self {
            config,
            db,
            redis,
            api: None,
        };
        Ok(context)
    }

    pub fn add_api(&mut self, api: Arc<OpenApi>) -> &mut Self {
        self.api = Some(api);
        self
    }
}
