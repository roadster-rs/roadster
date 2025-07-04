use crate::error::RoadsterResult;
use std::any::{Any, TypeId, type_name};
use std::collections::BTreeMap;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExtensionRegistryError {
    /// The provided [`AppService`] was already registered. Contains the [`AppService::name`]
    /// of the provided service.
    #[error("The provided `AppService` was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// Unable to find an [`AppService`] instance of the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to find an `AppService` instance of type `{0}`")]
    NotRegistered(String),

    /// Unable to downcast the registered instance to the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to downcast the registered instance of `AppService` to type `{0}`")]
    Downcast(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ExtensionRegistry {
    pub fn register<T>(&mut self, extension: T) -> RoadsterResult<&mut Self>
    where
        T: 'static + Send + Sync,
    {
        info!(name=%type_name::<T>(), "Registering extension");

        if self
            .extensions
            .insert(extension.type_id(), Box::new(extension))
            .is_some()
        {
            return Err(
                ExtensionRegistryError::AlreadyRegistered(type_name::<T>().to_owned()).into(),
            );
        }
        Ok(self)
    }

    pub fn get<T>(&self) -> RoadsterResult<&T>
    where
        T: 'static + Send + Sync,
    {
        let service = self
            .extensions
            .get(&TypeId::of::<T>())
            .ok_or_else(|| ExtensionRegistryError::NotRegistered(type_name::<T>().to_string()))?
            .downcast_ref::<T>()
            .ok_or_else(|| ExtensionRegistryError::Downcast(type_name::<T>().to_string()))?;
        Ok(service)
    }
}
