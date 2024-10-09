use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EmailError {
    #[cfg(feature = "email-smtp")]
    #[error(transparent)]
    Smtp(#[from] lettre::transport::smtp::Error),

    #[cfg(feature = "email")]
    #[error(transparent)]
    Address(#[from] lettre::address::AddressError),

    #[cfg(feature = "email")]
    #[error(transparent)]
    LettreError(#[from] lettre::error::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "email-smtp")]
impl From<lettre::transport::smtp::Error> for Error {
    fn from(value: lettre::transport::smtp::Error) -> Self {
        Self::Email(EmailError::from(value))
    }
}

#[cfg(feature = "email")]
impl From<lettre::address::AddressError> for Error {
    fn from(value: lettre::address::AddressError) -> Self {
        Self::Email(EmailError::from(value))
    }
}

#[cfg(feature = "email")]
impl From<lettre::error::Error> for Error {
    fn from(value: lettre::error::Error) -> Self {
        Self::Email(EmailError::from(value))
    }
}
