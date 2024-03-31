use crate::initializer::normalize_path::NormalizePathInitializer;
use crate::initializer::Initializer;

pub fn default_initializers<S>() -> Vec<Box<dyn Initializer<S>>> {
    vec![Box::new(NormalizePathInitializer)]
}
