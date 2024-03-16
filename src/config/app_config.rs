use anyhow::anyhow;
use config::Config;
use dotenvy::dotenv;
use serde_derive::{Deserialize, Serialize};

use crate::config::environment::Environment;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: Server,
    pub tracing: Tracing,
}

impl AppConfig {
    pub fn new() -> anyhow::Result<Self> {
        dotenv().ok();

        let environment = Environment::new()?;
        let environment: &'static str = environment.into();

        Config::builder()
            .add_source(config::File::with_name("config/default.toml"))
            .add_source(config::File::with_name(&format!(
                "config/{environment}.toml"
            )))
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
            .map_err(|err| anyhow!("Unable to deserialize app config: {err:?}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracing {
    pub level: String,
}
