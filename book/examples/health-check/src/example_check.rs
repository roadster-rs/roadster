use async_trait::async_trait;
use roadster::app::context::{AppContext, AppContextWeak};
use roadster::error::RoadsterResult;
use roadster::health::check::{CheckResponse, HealthCheck, Status};
use std::time::Duration;

pub struct ExampleCheck {
    state: AppContextWeak,
}

impl ExampleCheck {
    pub fn new(state: &AppContext) -> Self {
        Self {
            state: state.downgrade(),
        }
    }
}

#[async_trait]
impl HealthCheck for ExampleCheck {
    fn name(&self) -> String {
        "example".to_string()
    }

    fn enabled(&self) -> bool {
        // Custom health checks can be enabled/disabled via the app config
        // just like built-in checks
        if let Some(state) = self.state.upgrade() {
            state
                .config()
                .health_check
                .custom
                .get(&self.name())
                .map(|config| config.common.enabled(&state))
                .unwrap_or_else(|| state.config().health_check.default_enable)
        } else {
            false
        }
    }

    async fn check(&self) -> RoadsterResult<CheckResponse> {
        Ok(CheckResponse::builder()
            .status(Status::Ok)
            .latency(Duration::from_secs(0))
            .build())
    }
}
