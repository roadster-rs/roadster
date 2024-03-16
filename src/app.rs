use async_trait::async_trait;

use crate::app_context::AppContext;
use crate::config::app_config::AppConfig;
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
    type State: From<AppContext>;

    fn init_tracing(&self, config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
    }

    /// Convert the [AppContext] to the custom [Self::State] that will be used throughout the app.
    /// The conversion should mostly happen in a [From<AppContext>] implementation, but this
    /// method is provided in case there's any additional work that needs to be done that the
    /// consumer doesn't want to put in a [From<AppContext>] implementation. For example, any
    /// configuration that needs to happen in an async method.
    async fn context_to_state(&self, app_context: &AppContext) -> anyhow::Result<Self::State> {
        let state = Self::State::from(app_context.clone());
        Ok(state)
    }
}
