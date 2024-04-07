use aide::axum::ApiRouter;
use migration::Migrator;
use roadster::app::App as RoadsterApp;
use roadster::config::app_config::AppConfig;
use roadster::controller::default_routes;

use crate::app_state::AppState;
use crate::cli::AppCli;

const BASE: &str = "/api";

#[derive(Default)]
pub struct App;
impl RoadsterApp for App {
    type State = AppState;
    type Cli = AppCli;
    type M = Migrator;

    fn router(config: &AppConfig) -> ApiRouter<Self::State> {
        default_routes(BASE, config)
    }
}
