use crate::app_context::AppContext;
use crate::service::http::initializer::normalize_path::NormalizePathInitializer;
use crate::service::http::initializer::Initializer;
use std::collections::BTreeMap;

pub fn default_initializers<S>(
    context: &AppContext,
    state: &S,
) -> BTreeMap<String, Box<dyn Initializer<S>>> {
    let initializers: Vec<Box<dyn Initializer<S>>> = vec![Box::new(NormalizePathInitializer)];
    initializers
        .into_iter()
        .filter(|initializer| initializer.enabled(context, state))
        .map(|initializer| (initializer.name(), initializer))
        .collect()
}
