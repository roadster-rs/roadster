use crate::migrator::Migrator;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

type App = RoadsterApp<AppContext>;

fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .sea_orm_migrator(Migrator)
        .build()
}
