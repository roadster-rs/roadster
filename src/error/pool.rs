use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PoolError {
    #[error(transparent)]
    R2D2(#[from] r2d2::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<r2d2::Error> for Error {
    fn from(value: r2d2::Error) -> Self {
        Self::Pool(PoolError::from(value))
    }
}
