use crate::error::RoadsterResult;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use url::Url;

#[cfg(all(feature = "db-sea-orm", feature = "testing"))]
pub async fn create_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let conn = connection(uri).await?;
    let statement = create_database_statement(conn.get_database_backend(), db_name);
    conn.execute(statement).await?;
    conn.close().await?;
    Ok(())
}

pub async fn drop_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let conn = connection(uri).await?;
    let statement = drop_database_statement(conn.get_database_backend(), db_name);
    conn.execute(statement).await?;
    conn.close().await?;
    Ok(())
}

fn create_database_statement(backend: DbBackend, db_name: &str) -> Statement {
    // Todo: don't use string parameterization. It's mostly okay for now because we're not using
    //  user-provided input, and this is only used for tests, but it's still not great.
    //  I tried using `Statement::from_sql_and_values` but it wasn't working for some reason.
    Statement::from_string(backend, format!("CREATE DATABASE \"{db_name}\""))
}

fn drop_database_statement(backend: DbBackend, db_name: &str) -> Statement {
    // Todo: don't use string parameterization. It's mostly okay for now because we're not using
    //  user-provided input, and this is only used for tests, but it's still not great.
    //  I tried using `Statement::from_sql_and_values` but it wasn't working for some reason.
    Statement::from_string(
        backend,
        format!("DROP DATABASE IF EXISTS \"{db_name}\" WITH (FORCE)"),
    )
}

async fn connection(uri: &Url) -> RoadsterResult<DatabaseConnection> {
    let mut conn_options = sea_orm::ConnectOptions::new(uri.as_ref());
    conn_options.max_connections(1).connect_lazy(true);
    let conn = Database::connect(conn_options).await?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use sea_orm::DbBackend;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_database_statement() {
        let statement = super::create_database_statement(DbBackend::Postgres, "test");
        assert_debug_snapshot!(statement);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_database_statement() {
        let statement = super::drop_database_statement(DbBackend::Postgres, "test");
        assert_debug_snapshot!(statement);
    }
}
