use crate::example_async_source::ExampleAsyncSource;
use roadster::app::context::AppContext;
use roadster::app::RoadsterApp;
use roadster::util::empty::Empty;

pub mod example_async_source;

pub type App = RoadsterApp<AppContext, Empty>;

pub fn build_app() -> App {
    let builder = RoadsterApp::builder();

    let builder = builder.add_async_config_source(ExampleAsyncSource);

    let builder = builder.state_provider(|context| Ok(context));
    builder.build()
}
