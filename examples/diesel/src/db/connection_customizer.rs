use diesel_async::pooled_connection::{PoolError, PoolableConnection, RecyclingMethod};
use roadster::db::DieselPgConnAsync;
use std::pin::Pin;
use tracing::info;

#[derive(Debug)]
pub struct CustomConnectionCustomizer;

impl bb8::CustomizeConnection<DieselPgConnAsync, PoolError> for CustomConnectionCustomizer {
    fn on_acquire<'a>(
        &'a self,
        connection: &'a mut DieselPgConnAsync,
    ) -> Pin<Box<dyn Future<Output = Result<(), PoolError>> + Send + 'a>> {
        Box::pin(async {
            // Note: this `ping` is redundant with the `database.test-on-checkout` config field
            let result = connection.ping(&RecyclingMethod::Fast).await;
            let healthy = result.is_ok();
            info!(%healthy, "Connection acquired");
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(PoolError::QueryError(err)),
            }
        })
    }
}
