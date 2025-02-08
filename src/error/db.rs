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
    DieselPool(#[from] diesel::r2d2::Error),

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
impl From<diesel::r2d2::Error> for Error {
    fn from(value: diesel::r2d2::Error) -> Self {
        Self::Db(DbError::from(value))
    }
}
