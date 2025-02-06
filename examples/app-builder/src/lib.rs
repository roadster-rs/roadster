use crate::app_state::AppState;
use cfg_if::cfg_if;
use roadster::app::RoadsterApp;

pub mod api;
pub mod app_state;
#[cfg(feature = "cli")]
pub mod cli;
pub mod health;
pub mod lifecycle;
#[cfg(feature = "db-sea-orm")]
pub mod model;
pub mod worker;

cfg_if! {
if #[cfg(all(feature = "cli", feature = "db-sea-orm"))] {
    pub type App = RoadsterApp<AppState, cli::AppCli, migration::Migrator>;
} else if #[cfg(feature = "cli")] {
    pub type App = RoadsterApp<AppState, crate::cli::AppCli, roadster::util::empty::Empty>;
} else if #[cfg(feature = "db-sea-orm")] {
    pub type App = RoadsterApp<AppState, roadster::util::empty::Empty, migration::Migrator>;
} else {
    pub type App = RoadsterApp<AppState, roadster::util::empty::Empty, roadster::util::empty::Empty>;
}
}
