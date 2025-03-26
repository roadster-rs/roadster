use crate::cli::AppCli;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

pub mod cli;

pub type App = RoadsterApp<AppContext, AppCli>;

pub fn build_app() -> App {
    RoadsterApp::builder().state_provider(Ok).build()
}
