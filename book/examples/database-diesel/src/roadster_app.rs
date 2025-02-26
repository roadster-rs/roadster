use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

type App = RoadsterApp<AppContext>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .diesel_migrator::<roadster::db::DieselPgConn>(MIGRATIONS)
        .build()
}
