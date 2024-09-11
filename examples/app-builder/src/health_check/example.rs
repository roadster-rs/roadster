use async_trait::async_trait;
use roadster::error::RoadsterResult;
use roadster::health_check::{CheckResponse, HealthCheck, Status};
use std::time::Duration;

pub struct ExampleHealthCheck;

#[async_trait]
impl HealthCheck for ExampleHealthCheck {
    fn name(&self) -> String {
        "example".to_string()
    }

    fn enabled(&self) -> bool {
        true
    }

    async fn check(&self) -> RoadsterResult<CheckResponse> {
        Ok(CheckResponse::builder()
            .status(Status::Ok)
            .latency(Duration::from_secs(0))
            .build())
    }
}
