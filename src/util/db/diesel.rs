use crate::error::RoadsterResult;
use diesel::backend::Backend;
use diesel::connection::Connection;
use diesel::query_builder::*;
use diesel::result::QueryResult;
use diesel::RunQueryDsl;
use url::Url;

pub fn create_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let mut conn: AnyConnection = Connection::establish(uri.as_ref())?;
    let statement = CreateDatabaseStatement::new(db_name);
    RunQueryDsl::execute(statement, &mut conn)?;
    Ok(())
}

pub async fn drop_database(uri: &Url, db_name: &str) -> RoadsterResult<()> {
    let mut conn: AnyConnection = Connection::establish(uri.as_ref())?;
    let statement = DropDatabaseStatement::new(db_name);
    RunQueryDsl::execute(statement, &mut conn)?;
    Ok(())
}

#[derive(diesel::MultiConnection)]
enum AnyConnection {
    #[cfg(feature = "db-diesel-postgres")]
    Postgres(diesel::pg::PgConnection),
    #[cfg(feature = "db-diesel-mysql")]
    Mysql(diesel::mysql::MysqlConnection),
}

// Originally from https://github.com/diesel-rs/diesel/blob/7d8844547498407b6d5b8d1a37e695d35aa8c08b/diesel_cli/src/database.rs#L181
#[derive(Debug, Clone)]
struct DropDatabaseStatement {
    db_name: String,
}

impl DropDatabaseStatement {
    fn new(db_name: &str) -> Self {
        DropDatabaseStatement {
            db_name: db_name.to_owned(),
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for DropDatabaseStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("DROP DATABASE IF EXISTS ");
        out.push_identifier(&self.db_name)?;
        out.push_sql(" WITH (FORCE)");
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for DropDatabaseStatement {}

impl QueryId for DropDatabaseStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}

// Originally from https://github.com/diesel-rs/diesel/blob/7d8844547498407b6d5b8d1a37e695d35aa8c08b/diesel_cli/src/database.rs#L181
#[derive(Debug, Clone)]
struct CreateDatabaseStatement {
    db_name: String,
}

impl CreateDatabaseStatement {
    fn new(db_name: &str) -> Self {
        CreateDatabaseStatement {
            db_name: db_name.to_owned(),
        }
    }
}

impl<DB: Backend> QueryFragment<DB> for CreateDatabaseStatement {
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        out.push_sql("CREATE DATABASE ");
        out.push_identifier(&self.db_name)?;
        Ok(())
    }
}

impl<Conn> RunQueryDsl<Conn> for CreateDatabaseStatement {}

impl QueryId for CreateDatabaseStatement {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
