use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use roadster::app;
use roadster::app::{RoadsterApp, RoadsterAppBuilder};
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use roadster_diesel_example::api::http;
use roadster_diesel_example::app_state::AppState;
use roadster_diesel_example::App;

const BASE: &str = "/api";

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let builder: RoadsterAppBuilder<AppState, _> = RoadsterApp::builder()
        .state_provider(move |app_context| Ok(AppState::new(app_context)))
        .migrator(MIGRATIONS)
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

    app::run(app).await?;

    Ok(())
}
