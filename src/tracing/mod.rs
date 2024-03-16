use std::str::FromStr;

use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::app_config::AppConfig;

pub fn init_tracing(app_config: &AppConfig) -> anyhow::Result<()> {
    // Stdout Layer
    let stdout_layer = tracing_subscriber::fmt::layer();

    // Hide some noisy logs from traces
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::from_str(&app_config.tracing.level)?.into())
        .from_env()?
        .add_directive("h2=warn".parse()?)
        .add_directive("tower::buffer::worker=warn".parse()?);

    tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(stdout_layer)
        .try_init()?;

    Ok(())
}
