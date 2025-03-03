use async_trait::async_trait;
use config::{AsyncSource, ConfigError, Value};
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;

#[derive(Debug)]
pub struct ExampleAsyncSource;

#[async_trait]
impl AsyncSource for ExampleAsyncSource {
    async fn collect(&self) -> Result<config::Map<String, Value>, ConfigError> {
        let mut config = config::Map::new();

        /*
        Config fields can be set using the name of the field, where each level in the config
        is separated by a `.`

        For example, `service.sidekiq.redis.uri` overrides the `AppConfig#service#sidekiq#redis#uri` field.
        See: <https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.Redis.html#structfield.uri>

        Note: a hard-coded value is used here for demonstration purposes only. In a real application,
        an `AsyncSource` is intended to fetch the value from an external service, such as AWS or GCS
        secrets manager services.
         */
        config.insert(
            "service.sidekiq.redis.uri".into(),
            "redis://localhost:6379".into(),
        );

        Ok(config)
    }
}

type App = RoadsterApp<AppContext>;

fn build_app() -> App {
    let builder = RoadsterApp::builder();

    let builder = builder.add_async_config_source(ExampleAsyncSource);

    let builder = builder.state_provider(|context| Ok(context));
    builder.build()
}
