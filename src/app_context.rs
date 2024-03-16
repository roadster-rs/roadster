use crate::config::app_config::AppConfig;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: AppConfig,
}

impl AppContext {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let context = Self { config };
        Ok(context)
    }
}
