use async_trait::async_trait;

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

// Define a custom trait because the standard From/Into traits don't support async
#[async_trait]
pub trait FromAppContextRef {
    async fn from_app_context_ref(ctx: &AppContext) -> anyhow::Result<Box<Self>>;
}

// Define a custom trait because the standard From/Into traits don't support async
#[async_trait]
pub trait GetAppContextFromRef {
    async fn get_app_context(&self) -> anyhow::Result<AppContext>;
}

// Implement for `AppContext` so consumers can just use `AppContext` for their state if they want
#[async_trait]
impl FromAppContextRef for AppContext {
    async fn from_app_context_ref(ctx: &AppContext) -> anyhow::Result<Box<Self>> {
        Ok(Box::new(ctx.clone()))
    }
}

// Implement for `AppContext` so consumers can just use `AppContext` for their state if they want
#[async_trait]
impl GetAppContextFromRef for AppContext {
    async fn get_app_context(&self) -> anyhow::Result<AppContext> {
        Ok(self.clone())
    }
}
