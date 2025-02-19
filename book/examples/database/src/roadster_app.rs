use crate::migrator::Migrator;
use roadster::app::context::AppContext;
use roadster::app::RoadsterApp;

type App = RoadsterApp<AppContext>;

fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .sea_orm_migrator(Migrator)
        .build()
}
