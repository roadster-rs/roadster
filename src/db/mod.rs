#[cfg(feature = "db-sql")]
pub mod migration;

#[cfg(feature = "db-diesel-postgres")]
pub type DieselPgConn = diesel::pg::PgConnection;
#[cfg(feature = "db-diesel-mysql")]
pub type DieselMysqlConn = diesel::mysql::MysqlConnection;
#[cfg(feature = "db-diesel-sqlite")]
pub type DieselSqliteConn = diesel::sqlite::SqliteConnection;

#[cfg(feature = "db-diesel-postgres-pool")]
pub type DieselPgPool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselPgConn>>;
#[cfg(feature = "db-diesel-mysql-pool")]
pub type DieselMysqlPool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselMysqlConn>>;
#[cfg(feature = "db-diesel-sqlite-pool")]
pub type DieselSqlitePool = r2d2::Pool<diesel::r2d2::ConnectionManager<DieselSqliteConn>>;

#[cfg(feature = "db-diesel-postgres-pool-async")]
pub type DieselPgConnAsync = diesel_async::AsyncPgConnection;
#[cfg(feature = "db-diesel-mysql-pool-async")]
pub type DieselMysqlConnAsync = diesel_async::AsyncMysqlConnection;

#[cfg(feature = "db-diesel-postgres-pool-async")]
pub type DieselPgPoolAsync = diesel_async::pooled_connection::bb8::Pool<DieselPgConnAsync>;
#[cfg(feature = "db-diesel-mysql-pool-async")]
pub type DieselMysqlPoolAsync = diesel_async::pooled_connection::bb8::Pool<DieselMysqlConnAsync>;
