use async_trait::async_trait;
use config::{AsyncSource, ConfigError, Map, Value};

#[derive(Debug)]
pub struct ExampleAsyncSource;

#[async_trait]
impl AsyncSource for ExampleAsyncSource {
    async fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let mut config = Map::new();

        /*
        Config fields can be set using the name of the field, where each level in the config
        is separated by a `.`

        For example, `database.uri` overrides the `AppConfig#database#uri` field.
        See: <https://docs.rs/roadster/latest/roadster/config/database/struct.Database.html#structfield.uri>

        Note: a hard-coded value is used here for demonstration purposes only. In a real application,
        an `AsyncSource` is intended to fetch the value from an external service, such as AWS or GCS
        secrets manager services.
        */
        config.insert(
            "database.uri".into(),
            "postgres://roadster:roadster@localhost:5432/example_dev".into(),
        );

        /*
        And `service.sidekiq.redis.uri` overrides the `AppConfig#service#sidekiq#redis#uri` field.
        See: <https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.Redis.html#structfield.uri>

        Note: a hard-coded value is used here for demonstration purposes only. In a real application,
        an `AsyncSource` is intended to fetch the value from an external service, such as AWS or GCS
        secrets manager services.
         */
        config.insert(
            "service.worker.sidekiq.redis.uri".into(),
            "redis://localhost:6379".into(),
        );

        Ok(config)
    }
}
