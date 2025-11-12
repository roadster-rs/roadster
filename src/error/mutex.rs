use crate::error::Error;
use std::any::type_name;
use std::ops::Deref;
use std::sync::PoisonError;

#[derive(Debug, derive_more::Deref)]
pub struct MutexType(String);

#[derive(Debug, derive_more::Deref)]
pub struct MutexErrMsg(String);

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MutexError {
    #[error(
        "Unable to acquire mutex `{}`; the mutex is poisoned. Err: {}",
        .0.0.deref(),
        .0.1.deref()
    )]
    Poison((MutexType, MutexErrMsg)),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl<T> From<PoisonError<T>> for Error {
    fn from(value: PoisonError<T>) -> Self {
        MutexError::Poison((
            MutexType(type_name::<T>().to_string()),
            MutexErrMsg(value.to_string()),
        ))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use std::sync::PoisonError;

    struct FooStruct;

    #[test]
    fn from_poison_error() {
        let error = PoisonError::new(FooStruct);
        let error = crate::error::Error::from(error);
        assert_debug_snapshot!(error.to_string());
    }
}
