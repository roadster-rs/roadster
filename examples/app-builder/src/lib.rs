use crate::app_state::AppState;
use cfg_if::cfg_if;
use roadster::app::RoadsterApp;

pub mod api;
pub mod app_state;
#[cfg(feature = "cli")]
pub mod cli;
pub mod health_check;
pub mod lifecycle;
#[cfg(feature = "db-sql")]
pub mod model;
pub mod worker;

cfg_if! {
if #[cfg(all(feature = "cli", feature = "db-sql"))] {
    pub type App = RoadsterApp<AppState, cli::AppCli, migration::Migrator>;
} else if #[cfg(feature = "cli")] {
    pub type App = RoadsterApp<AppState, crate::cli::AppCli>;
} else if #[cfg(feature = "db-sql")] {
    pub type App = RoadsterApp<AppState, migration::Migrator>;
} else {
    pub type App = RoadsterApp<AppState>;
}
}
