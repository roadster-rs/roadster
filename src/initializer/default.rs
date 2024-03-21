use crate::initializer::normalize_path::NormalizePathInitializer;
use crate::initializer::Initializer;

pub fn default_initializers() -> Vec<Box<dyn Initializer>> {
    vec![Box::new(NormalizePathInitializer)]
}
