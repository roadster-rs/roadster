use crate::App;
use crate::app_state::AppState;
use roadster::lifecycle::AppLifecycleHandler;

pub struct ExampleLifecycleHandler {
    name: String,
}

impl ExampleLifecycleHandler {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl AppLifecycleHandler<App, AppState> for ExampleLifecycleHandler {
    fn name(&self) -> String {
        self.name.clone()
    }
}
