use crate::config::app_config::AppConfig;
use aide::openapi::OpenApi;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: AppConfig,
    pub api: Option<Arc<OpenApi>>,
}

impl AppContext {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        let context = Self { config, api: None };
        Ok(context)
    }

    pub fn add_api(&mut self, api: Arc<OpenApi>) -> &mut Self {
        self.api = Some(api);
        self
    }
}
