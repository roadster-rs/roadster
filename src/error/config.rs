use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl From<config::ConfigError> for Error {
    fn from(value: config::ConfigError) -> Self {
        Self::Config(ConfigError::from(value))
    }
}
