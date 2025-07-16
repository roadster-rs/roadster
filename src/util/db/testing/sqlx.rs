use crate::error::RoadsterResult;
use sqlx::postgres::PgConnectOptions;
use sqlx::{ConnectOptions, PgConnection};
use url::Url;

pub async fn create_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let mut connection = connection(uri).await?;
    sqlx::query(&create_database_statement(db_name))
        .execute(&mut connection)
        .await?;
    Ok(())
}

pub async fn drop_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let mut connection = connection(uri).await?;
    sqlx::query(&drop_database_statement(db_name))
        .execute(&mut connection)
        .await?;
    Ok(())
}

async fn connection(uri: &Url) -> RoadsterResult<PgConnection> {
    let connection = PgConnectOptions::from_url(uri)?;
    let connection = connection.connect().await?;
    Ok(connection)
}

fn create_database_statement(db_name: &str) -> String {
    // Todo: don't use string parameterization. It's mostly okay for now because we're not using
    //  user-provided input, and this is only used for tests, but it's still not great.
    format!("CREATE DATABASE \"{db_name}\"")
}

fn drop_database_statement(db_name: &str) -> String {
    // Todo: don't use string parameterization. It's mostly okay for now because we're not using
    //  user-provided input, and this is only used for tests, but it's still not great.
    format!("DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)")
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_database_statement() {
        let statement = super::create_database_statement("test");
        assert_snapshot!(statement);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_database_statement() {
        let statement = super::drop_database_statement("test");
        assert_snapshot!(statement);
    }
}
