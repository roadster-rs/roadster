use crate::app_state::AppState;
use crate::App;
use roadster::lifecycle::AppLifecycleHandler;

pub struct ExampleLifecycleHandler;

impl AppLifecycleHandler<App, AppState> for ExampleLifecycleHandler {
    fn name(&self) -> String {
        "example".to_string()
    }
}
