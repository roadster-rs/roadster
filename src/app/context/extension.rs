//! Registry of custom context to include in the [`crate::app::context::AppContext`].

use std::any::{Any, TypeId, type_name};
use std::collections::BTreeMap;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExtensionRegistryError {
    /// The provided context was already registered. Contains the [`type_name`]
    /// of the provided context.
    #[error("The provided `context` was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// Unable to find a context instance of the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to find an context instance of type `{0}`")]
    NotRegistered(String),

    /// Unable to downcast the registered instance to the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to downcast the registered context instance to type `{0}`")]
    Downcast(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Registry of custom context to include in the [`crate::app::context::AppContext`].
///
/// # Examples
/// ```
/// # use roadster::app::context::extension::ExtensionRegistry;
/// # let mut registry = ExtensionRegistry::default();
/// #
/// struct CustomContext {
///     foo: String,
/// }
/// registry.register(CustomContext { foo: String::from("foo")}).unwrap();
///
/// let value = registry.get::<CustomContext>().unwrap();
/// assert_eq!(value.foo, "foo");
///
/// let result = registry.register(CustomContext { foo: String::from("bar")});
/// assert!(result.is_err());
/// ```
#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ExtensionRegistry {
    /// Register the given extension value. Only a single value of any given type can be registered;
    /// if the same type is already registered, an error will be returned.
    pub fn register<T>(&mut self, extension: T) -> Result<&mut Self, ExtensionRegistryError>
    where
        T: 'static + Send + Sync,
    {
        info!(name=%type_name::<T>(), "Registering extension");

        if self
            .extensions
            .insert(extension.type_id(), Box::new(extension))
            .is_some()
        {
            return Err(ExtensionRegistryError::AlreadyRegistered(
                type_name::<T>().to_owned(),
            ));
        }
        Ok(self)
    }

    /// Get a previously registered value by its type. If no value of the specified type has been
    /// registered, an error will be returned.
    ///
    /// # Examples
    /// ```
    /// # use roadster::app::context::extension::ExtensionRegistry;
    /// # let mut registry = ExtensionRegistry::default();
    /// let result = registry.get::<String>();
    /// assert!(result.is_err());
    /// ```
    pub fn get<T>(&self) -> Result<&T, ExtensionRegistryError>
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

#[cfg(test)]
mod tests {
    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_and_get() {
        let mut registry = super::ExtensionRegistry::default();

        registry.register("Foo".to_owned()).unwrap();

        assert_eq!("Foo", registry.get::<String>().unwrap());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_duplicate() {
        let mut registry = super::ExtensionRegistry::default();

        registry.register("Foo".to_owned()).unwrap();
        assert!(registry.register("Foo".to_string()).is_err());
    }
}
