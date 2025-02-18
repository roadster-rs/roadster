use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DbError {
    #[cfg(feature = "db-sea-orm")]
    #[error(transparent)]
    SeaOrm(#[from] sea_orm::DbErr),

    #[cfg(feature = "db-diesel")]
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),

    #[cfg(feature = "db-diesel")]
    #[error(transparent)]
    DieselConnection(#[from] diesel::ConnectionError),

    #[cfg(feature = "db-diesel")]
    #[error(transparent)]
    DieselMigration(#[from] diesel_migrations::MigrationError),

    #[cfg(feature = "db-diesel-pool")]
    #[error(transparent)]
    DieselR2D2(#[from] r2d2::Error),

    #[cfg(feature = "db-diesel-pool-async")]
    #[error(transparent)]
    DieselAsyncPool(#[from] diesel_async::pooled_connection::PoolError),

    #[cfg(feature = "db-diesel-pool-async")]
    #[error(transparent)]
    DieselAsyncBb8Pool(#[from] diesel_async::pooled_connection::bb8::RunError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "db-sea-orm")]
impl From<sea_orm::DbErr> for Error {
    fn from(value: sea_orm::DbErr) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel")]
impl From<diesel::result::Error> for Error {
    fn from(value: diesel::result::Error) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel")]
impl From<diesel::ConnectionError> for Error {
    fn from(value: diesel::ConnectionError) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel")]
impl From<diesel_migrations::MigrationError> for Error {
    fn from(value: diesel_migrations::MigrationError) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel-pool")]
impl From<r2d2::Error> for Error {
    fn from(value: r2d2::Error) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel-pool-async")]
impl From<diesel_async::pooled_connection::PoolError> for Error {
    fn from(value: diesel_async::pooled_connection::PoolError) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel-pool-async")]
impl From<diesel_async::pooled_connection::bb8::RunError> for Error {
    fn from(value: diesel_async::pooled_connection::bb8::RunError) -> Self {
        Self::Db(DbError::from(value))
    }
}
