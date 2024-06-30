use crate::error::RoadsterResult;
use anyhow::anyhow;
use serde_derive::{Deserialize, Serialize};
use std::net::SocketAddr;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Address {
    pub host: String,
    pub port: u32,
}

impl Address {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
    pub fn socket_addr(&self) -> RoadsterResult<SocketAddr> {
        let addr = self
            .url()
            .parse()
            .map_err(|e| anyhow!("Unable to parse app url to a SocketAddr: {e}"))?;
        Ok(addr)
    }
}
