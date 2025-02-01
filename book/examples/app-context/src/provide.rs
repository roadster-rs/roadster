use roadster::app::context::{AppContext, ProvideRef};
use roadster::error::RoadsterResult;
use sea_orm::DatabaseConnection;

pub async fn check_db_health(context: AppContext) -> RoadsterResult<()> {
    // `ping_db` can be called with `AppContext` in order to use the actual `DatabaseConnection`
    // in production
    ping_db(context).await
}

/// Example app method that takes a [`ProvideRef`] in order to allow using a mocked
/// [`DatabaseConnection`] in tests.
async fn ping_db(db: impl ProvideRef<DatabaseConnection>) -> RoadsterResult<()> {
    db.provide().ping().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use roadster::app::context::MockProvideRef;
    use sea_orm::{DatabaseBackend, DatabaseConnection, MockDatabase};

    #[tokio::test]
    async fn db_ping() {
        let db = MockDatabase::new(DatabaseBackend::Postgres).into_connection();
        let mut db_provider = MockProvideRef::<DatabaseConnection>::new();
        db_provider.expect_provide().return_const(db);

        super::ping_db(db_provider).await.unwrap();
    }
}
