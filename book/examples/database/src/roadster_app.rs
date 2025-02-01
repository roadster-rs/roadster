use crate::migrator::Migrator;
use roadster::app::context::AppContext;
use roadster::app::RoadsterApp;
use roadster::util::empty::Empty;

type App = RoadsterApp<AppContext, Empty, Migrator>;

fn build_app() -> App {
    RoadsterApp::builder()
        .state_provider(|context| Ok(context))
        .build()
}
