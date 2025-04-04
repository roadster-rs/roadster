use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::app::{PreparedAppWithoutCli, RoadsterApp};
use roadster::error::RoadsterResult;
use roadster::lifecycle::AppLifecycleHandler;

pub struct ExampleLifecycleHandler;

#[async_trait]
impl AppLifecycleHandler<RoadsterApp<AppContext>, AppContext> for ExampleLifecycleHandler {
    fn name(&self) -> String {
        "example".to_owned()
    }

    fn enabled(&self, state: &AppContext) -> bool {
        // Custom lifecycle handlers can be enabled/disabled via the app config
        // just like built-in handlers
        state
            .config()
            .lifecycle_handler
            .custom
            .get(&self.name())
            .map(|config| config.common.enabled(&state))
            .unwrap_or_else(|| state.config().health_check.default_enable)
    }

    fn priority(&self, state: &AppContext) -> i32 {
        state
            .config()
            .lifecycle_handler
            .custom
            .get(&self.name())
            .map(|config| config.common.priority)
            .unwrap_or_default()
    }

    async fn before_health_checks(
        &self,
        _prepared_app: &PreparedAppWithoutCli<RoadsterApp<AppContext>, AppContext>,
    ) -> RoadsterResult<()> {
        todo!("Implement in order to initialize some state before health checks run")
    }

    async fn before_services(
        &self,
        _prepared_app: &PreparedAppWithoutCli<RoadsterApp<AppContext>, AppContext>,
    ) -> RoadsterResult<()> {
        todo!("Implement in order to initialize some state before the app's services are started")
    }

    async fn on_shutdown(&self, _state: &AppContext) -> RoadsterResult<()> {
        todo!("Implement in order to perform any necessary clean up on app shutdown")
    }
}
