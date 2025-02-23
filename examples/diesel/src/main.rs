use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use roadster::app;
use roadster::app::{RoadsterApp, RoadsterAppBuilder};
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster_diesel_example::api::http;
use roadster_diesel_example::app_state::AppState;
use roadster_diesel_example::{build_app, App};

const BASE: &str = "/api";

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let app = build_app();

    app::run(app).await?;

    Ok(())
}
