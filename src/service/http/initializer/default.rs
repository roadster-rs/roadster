#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::initializer::normalize_path::NormalizePathInitializer;
use crate::service::http::initializer::Initializer;
use std::collections::BTreeMap;

pub fn default_initializers<S: Send + Sync + 'static>(
    context: &AppContext<S>,
) -> BTreeMap<String, Box<dyn Initializer<S>>> {
    let initializers: Vec<Box<dyn Initializer<S>>> = vec![Box::new(NormalizePathInitializer)];
    initializers
        .into_iter()
        .filter(|initializer| initializer.enabled(context))
        .map(|initializer| (initializer.name(), initializer))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 0)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_initializers(#[case] default_enable: bool, #[case] expected_size: usize) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.initializer.default_enable = default_enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        // Act
        let middleware = super::default_initializers(&context);

        // Assert
        assert_eq!(middleware.len(), expected_size);
    }
}
