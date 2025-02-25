use async_trait::async_trait;
use diesel_async::pooled_connection::{PoolError, PoolableConnection, RecyclingMethod};
use roadster::db::DieselPgConnAsync;
use tracing::info;

#[derive(Debug)]
pub struct CustomConnectionCustomizer;

#[async_trait]
impl bb8_8::CustomizeConnection<DieselPgConnAsync, PoolError> for CustomConnectionCustomizer {
    async fn on_acquire(&self, connection: &mut DieselPgConnAsync) -> Result<(), PoolError> {
        // Note: this `ping` is redundant with the `database.test-on-checkout` config field
        let result = connection.ping(&RecyclingMethod::Fast).await;
        let healthy = result.is_ok();
        info!(%healthy, "Connection acquired");
        match result {
            Ok(_) => Ok(()),
            Err(err) => Err(PoolError::QueryError(err)),
        }
    }
}
