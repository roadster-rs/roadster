use crate::app_state::{AppState, AppStateWeak};
use async_trait::async_trait;
use roadster::error::RoadsterResult;
use roadster::health::check::{CheckResponse, ErrorData, HealthCheck, Status};
use std::time::Duration;
use tracing::error;

/// An example [`HealthCheck`] implementation demonstrating how to use [`AppStateWeak`] in order
/// to prevent a reference cycle.
///
/// The [`roadster::app::context::AppContext`] contained in the regular [`AppState`] contains
/// the list of registered [`HealthCheck`]s. This causes a reference cycle unless we use a weak
/// pointer to break the cycle. Note that in a production system, this reference cycle is not likely
/// to cause any actual issues because the [`roadster::app::context::AppContext`] and the
/// [`HealthCheck`]s should all live for the lifetime of your application process anyway. However,
/// it can cause issues if you're using Roadster's `test-containers` feature -- the test containers
/// are only cleaned up when the [`roadster::app::context::AppContext`] is dropped, but a reference
/// cycle can cause the [`roadster::app::context::AppContext`] to never be dropped, so the
/// test containers would never be cleaned up. This may not be an immediate issue, but it
/// could cause your local development environment to become clogged with a bunch of unnecessary
/// docker containers.
pub struct ExampleHealthCheck {
    state: AppStateWeak,
}

impl ExampleHealthCheck {
    pub fn new(state: &AppState) -> Self {
        Self {
            state: state.downgrade(),
        }
    }
}

#[async_trait]
impl HealthCheck for ExampleHealthCheck {
    fn name(&self) -> String {
        "example".to_string()
    }

    fn enabled(&self) -> bool {
        let state = match self.state.upgrade() {
            Some(state) => state,
            None => return false,
        };
        state
            .app_context
            .config()
            .health_check
            .custom
            .get(&self.name())
            .map(|config| config.common.enabled(&state))
            .unwrap_or(state.app_context.config().health_check.default_enable)
    }

    async fn check(&self) -> RoadsterResult<CheckResponse> {
        let state = self.state.upgrade();

        let response = match state {
            Some(_state) => CheckResponse::builder()
                .status(Status::Ok)
                .latency(Duration::from_millis(0))
                .custom("Example health check successful")
                .build(),
            None => {
                let msg = "AppState missing; is the app shutting down?".to_string();
                error!(msg);
                CheckResponse::builder()
                    .status(Status::Err(ErrorData::builder().msg(msg).build()))
                    .latency(Duration::from_secs(0))
                    .build()
            }
        };

        Ok(response)
    }
}
