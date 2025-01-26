use crate::app::App;
use crate::app_state::AppState;
use roadster::lifecycle::AppLifecycleHandler;

pub struct ExampleLifecycleHandler;

impl AppLifecycleHandler<App, AppState> for ExampleLifecycleHandler {
    fn name(&self) -> String {
        "example".to_string()
    }
}
