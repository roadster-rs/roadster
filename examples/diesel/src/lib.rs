use crate::app_state::AppState;
use roadster::app::RoadsterApp;

pub mod api;
pub mod app_state;
pub mod cli;

pub type App = RoadsterApp<AppState, cli::AppCli, roadster::util::empty::Empty>;
