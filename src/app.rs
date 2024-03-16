use crate::config::app_config::AppConfig;
use crate::tracing::init_tracing;

pub async fn start() -> anyhow::Result<()> {
    let config = AppConfig::new()?;
    init_tracing(&config)?;

    Ok(())
}
