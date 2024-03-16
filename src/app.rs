use async_trait::async_trait;

use crate::config::app_config::AppConfig;
use crate::context::{AppContext, FromAppContextRef, GetAppContextFromRef};
use crate::tracing::init_tracing;

pub async fn start<A>() -> anyhow::Result<()>
where
    A: App + Default + Sync,
{
    let config = AppConfig::new()?;

    let app = A::default();
    app.init_tracing(&config)?;

    let context = AppContext::new(config).await?;
    let _context = app.context_to_state(&context).await?;

    Ok(())
}

#[async_trait]
pub trait App {
    type State: FromAppContextRef + GetAppContextFromRef;

    fn init_tracing(&self, config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
    }

    async fn context_to_state(&self, app_context: &AppContext) -> anyhow::Result<Self::State> {
        let state = Self::State::from_app_context_ref(app_context).await?;
        Ok(*state)
    }
}
