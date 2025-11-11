use crate::app::App;
use crate::app_state::AppState;
use roadster::lifecycle::AppLifecycleHandler;
use std::convert::Infallible;

pub struct ExampleLifecycleHandler;

impl AppLifecycleHandler<App, AppState> for ExampleLifecycleHandler {
    type Error = Infallible;

    fn name(&self) -> String {
        "example".to_string()
    }
}
