use crate::app::context::AppContext;
use crate::service::http::initializer::Initializer;
use axum::Router;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use validator::Validate;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct NormalizePathConfig {}

pub struct NormalizePathInitializer;

impl<S> Initializer<S> for NormalizePathInitializer
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        "normalize-path".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .initializer
            .normalize_path
            .common
            .enabled(state)
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .initializer
            .normalize_path
            .common
            .priority
    }

    /// Add the [`NormalizePathLayer`] to handle a trailing `/` at the end of URIs.
    ///
    /// Normally, adding a layer via the axum [`Router::layer`] method causes the layer to run
    /// after routing has already completed. This means the [`NormalizePathLayer`] would not
    /// normalize the uri for the purposes of routing, which defeats the point of the layer.
    /// The workaround is to wrap the entire router with [`NormalizePathLayer`], which is why this
    /// middleware is applied in an [`Initializer`] instead of as a normal
    /// [`crate::service::http::middleware::Middleware`] -- this way, the [`NormalizePathLayer`]
    /// is applied after all the routes and normal middleware have been applied.
    ///
    /// See: <https://docs.rs/axum/latest/axum/middleware/index.html#rewriting-request-uri-in-middleware>
    fn before_serve(&self, router: Router, _state: &S) -> Result<Router, Self::Error> {
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let router = Router::new().fallback_service(router);
        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.initializer.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .initializer
            .normalize_path
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let initializer = NormalizePathInitializer;

        // Act/Assert
        assert_eq!(initializer.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 10000)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .initializer
                .normalize_path
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let initializer = NormalizePathInitializer;

        // Act/Assert
        assert_eq!(initializer.priority(&context), expected_priority);
    }
}
