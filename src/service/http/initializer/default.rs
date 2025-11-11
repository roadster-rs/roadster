use crate::app::context::AppContext;
use crate::service::http::initializer::Initializer;
use crate::service::http::initializer::normalize_path::NormalizePathInitializer;
use axum_core::extract::FromRef;
use std::collections::BTreeMap;

pub fn default_initializers<S>(
    state: &S,
) -> BTreeMap<String, Box<dyn Initializer<S, Error = crate::error::Error>>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let initializers: Vec<Box<dyn Initializer<S, Error = crate::error::Error>>> =
        vec![Box::new(NormalizePathInitializer)];

    initializers
        .into_iter()
        .filter(|initializer| initializer.enabled(state))
        .map(|initializer| (initializer.name(), initializer))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 0)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_initializers(#[case] default_enable: bool, #[case] expected_size: usize) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.initializer.default_enable = default_enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act
        let middleware = super::default_initializers(&context);

        // Assert
        assert_eq!(middleware.len(), expected_size);
    }
}
