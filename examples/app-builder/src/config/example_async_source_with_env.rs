use async_trait::async_trait;
use config::{AsyncSource, ConfigError, Map, Value};
use roadster::config::environment::Environment;

#[derive(Debug)]
pub struct ExampleAsyncSourceWithEnv {
    environment: Environment,
}

impl ExampleAsyncSourceWithEnv {
    pub fn new(environment: &Environment) -> Self {
        Self {
            environment: environment.clone(),
        }
    }
}

#[async_trait]
impl AsyncSource for ExampleAsyncSourceWithEnv {
    async fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let mut config = Map::new();

        /*
        Note: Hard-coded values are used here for demonstration purposes only. In a real application,
        an `AsyncSource` is intended to fetch the value from an external service, such as AWS or GCS
        secrets manager services.
        */
        let uri = match self.environment {
            Environment::Development | Environment::Test => {
                "postgres://roadster:roadster@localhost:5432/example_dev"
            }
            Environment::Production => "postgres://roadster:roadster@localhost:5432/example_prod",
            _ => unimplemented!(),
        };

        config.insert("database.uri".into(), uri.into());

        Ok(config)
    }
}
