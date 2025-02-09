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
    DieselPool(#[from] diesel_async::pooled_connection::PoolError),

    #[cfg(feature = "db-diesel")]
    #[error(transparent)]
    DieselBb8Pool(#[from] diesel_async::pooled_connection::bb8::RunError),

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
impl From<diesel_async::pooled_connection::PoolError> for Error {
    fn from(value: diesel_async::pooled_connection::PoolError) -> Self {
        Self::Db(DbError::from(value))
    }
}

#[cfg(feature = "db-diesel")]
impl From<diesel_async::pooled_connection::bb8::RunError> for Error {
    fn from(value: diesel_async::pooled_connection::bb8::RunError) -> Self {
        Self::Db(DbError::from(value))
    }
}
