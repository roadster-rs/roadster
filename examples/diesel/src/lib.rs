use crate::api::http;
use crate::app_state::AppState;
use crate::db::connection_customizer::CustomConnectionCustomizer;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use roadster::app::{RoadsterApp, RoadsterAppBuilder};
use roadster::service::http::service::HttpService;

pub mod api;
pub mod app_state;
pub mod cli;
pub mod db;
pub mod models;
pub mod schema;

pub type App = RoadsterApp<AppState, cli::AppCli>;

const BASE: &str = "/api";

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub fn build_app() -> App {
    let builder: RoadsterAppBuilder<AppState, _> = RoadsterApp::builder()
        .state_provider(move |app_context| Ok(AppState::new(app_context)))
        /*
        The Db connection customizer can either be provided directly with
        `diesel_*_connection_customizer` or via the
        `diesel_*_connection_customizer_provider` callback. Note that the two approaches are
        mutually exclusive, with the `diesel_*_connection_customizer` method taking priority.
         */
        .diesel_pg_async_connection_customizer(CustomConnectionCustomizer)
        .diesel_pg_async_connection_customizer_provider(|_config| {
            Ok(Box::new(CustomConnectionCustomizer))
        })
        /*
        Roadster can automatically run the app's DB migrations on start up. Simply provide
        the app's migrator instance (something that implements diesel's `MigrationSource`), and
        specify the connection type to use to run the migrations.
         */
        .diesel_migrator::<diesel::pg::PgConnection>(MIGRATIONS)
        .add_service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        HttpService::builder(Some(BASE), state).api_router(http::routes(BASE)),
                    )
                    .await?;
                Ok(())
            })
        });

    let app: App = builder.build();

    app
}
