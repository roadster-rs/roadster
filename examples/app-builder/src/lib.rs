use crate::app_state::AppState;
use cfg_if::cfg_if;
use roadster::app::RoadsterApp;

pub mod api;
pub mod app_state;
pub mod config;
pub mod health;
pub mod lifecycle;
#[cfg(feature = "db-sea-orm")]
pub mod model;
pub mod worker;

cfg_if! {
if #[cfg(feature = "cli")] {
    pub type App = RoadsterApp<AppState, api::cli::AppCli>;
} else {
    pub type App = RoadsterApp<AppState, roadster::util::empty::Empty>;
}
}
