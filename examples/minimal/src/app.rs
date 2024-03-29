// The RoadsterApp trait uses `AppContext`, so allow an exception in order to implement the trait.
#![allow(clippy::disallowed_types)]

use aide::axum::ApiRouter;
use roadster::app::App as RoadsterApp;
use roadster::config::app_config::AppConfig;
use roadster::controller::default_routes;

use crate::app_state::AppState;

const BASE: &str = "/api";

#[derive(Default)]
pub struct App;
impl RoadsterApp for App {
    type State = AppState;

    fn router(config: &AppConfig) -> ApiRouter<Self::State> {
        default_routes(BASE, config)
    }
}
