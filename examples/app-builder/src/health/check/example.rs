use async_trait::async_trait;
use roadster::error::RoadsterResult;
use roadster::health::check::{CheckResponse, HealthCheck, Status};
use std::convert::Infallible;
use std::time::Duration;

pub struct ExampleHealthCheck {
    pub name: String,
}

impl ExampleHealthCheck {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl HealthCheck for ExampleHealthCheck {
    type Error = Infallible;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self) -> bool {
        true
    }

    async fn check(&self) -> Result<CheckResponse, Self::Error> {
        Ok(CheckResponse::builder()
            .status(Status::Ok)
            .latency(Duration::from_secs(0))
            .build())
    }
}
