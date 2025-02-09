use crate::app_state::AppState;
use roadster::app::RoadsterApp;

pub mod api;
pub mod app_state;
pub mod cli;
pub mod models;
pub mod schema;

pub type App = RoadsterApp<AppState, cli::AppCli>;
