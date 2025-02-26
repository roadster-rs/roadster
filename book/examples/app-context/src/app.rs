use crate::state::CustomState;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::extract::FromRef;
use roadster::app::RoadsterApp;
use roadster::app::context::{AppContext, AppContextWeak};
use roadster::error::RoadsterResult;
use roadster::health::check::{CheckResponse, HealthCheck, Status};
use std::time::Duration;

pub type App = RoadsterApp<CustomState>;

pub struct ExampleHealthCheck {
    // Prevent reference cycle because the `ExampleHealthCheck` is also stored in the `AppContext`
    context: AppContextWeak,
}

#[async_trait]
impl HealthCheck for ExampleHealthCheck {
    fn name(&self) -> String {
        "example".to_string()
    }

    fn enabled(&self) -> bool {
        true
    }

    async fn check(&self) -> RoadsterResult<CheckResponse> {
        // Upgrade the `AppContext` in order to use it
        let _context = self
            .context
            .upgrade()
            .ok_or_else(|| anyhow!("Could not upgrade AppContextWeak"))?;

        Ok(CheckResponse::builder()
            .status(Status::Ok)
            .latency(Duration::from_secs(0))
            .build())
    }
}

pub fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| {
            Ok(CustomState {
                context,
                custom_field: "Custom Field".to_string(),
            })
        })
        .add_health_check_provider(|registry, state| {
            // Downgrade the context before providing it to the `ExampleHealthCheck`
            let context = AppContext::from_ref(state).downgrade();
            registry.register(ExampleHealthCheck { context })
        })
        .build()
}
