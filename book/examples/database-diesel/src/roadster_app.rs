use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use roadster::app::context::AppContext;
use roadster::app::RoadsterApp;

type App = RoadsterApp<AppContext>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .diesel_migrator(MIGRATIONS)
        .build()
}
