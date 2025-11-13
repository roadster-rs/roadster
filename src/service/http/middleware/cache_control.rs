use crate::app::context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderValue, Response};
use axum_core::body::Body;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;
use std::time::Duration;
use tower_http::set_header::SetResponseHeaderLayer;
use validator::Validate;

#[serde_as]
#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct CacheControlConfig {
    /// The `max-age` to set in the `cache-control` header. The header will only be set
    /// for responses who's content type matches an entry in the `content_types` field.
    #[serde_as(as = "serde_with::DurationSeconds")]
    pub max_age: Duration,

    /// The content types to set the `cache-control` header for and any custom configuration for
    /// each content type.
    #[serde(default)]
    #[validate(nested)]
    pub content_types: BTreeMap<String, ContentTypeConfig>,
}

#[serde_as]
#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct ContentTypeConfig {
    /// The `max-age` to set in the `cache-control` header for this content type. If not provided,
    /// the `max-age` from [`CacheControlConfig`] will be used.
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub max_age: Option<Duration>,
}

pub struct CacheControlMiddleware;
impl<S> Middleware<S> for CacheControlMiddleware
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        "cache-control".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        let config = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .cache_control;

        config.common.enabled(state) && !config.custom.content_types.is_empty()
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .cache_control
            .common
            .priority
    }

    fn install(&self, state: &S, router: Router) -> Result<Router, Self::Error> {
        let state = state.clone();
        let layer = SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            move |response: &Response<Body>| {
                let context = AppContext::from_ref(&state);
                let config = &context
                    .config()
                    .service
                    .http
                    .custom
                    .middleware
                    .cache_control;
                let max_age = config.custom.max_age;

                let headers = response.headers();
                headers
                    .get(CONTENT_TYPE)
                    .and_then(|content_type| content_type.to_str().ok())
                    .and_then(|content_type| config.custom.content_types.get(content_type))
                    .map(|config| config.max_age.unwrap_or(max_age))
                    .and_then(|max_age| {
                        HeaderValue::from_str(&format!("max-age={}", max_age.as_secs())).ok()
                    })
            },
        );

        let router = router.layer(layer);

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("text/css"), true)]
    #[case(false, Some(true), Some("text/css"), true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] content_type: Option<&str>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        let cache_control_config = &mut config.service.http.custom.middleware.cache_control;
        if let Some(content_type) = content_type {
            cache_control_config
                .custom
                .content_types
                .insert(content_type.to_string(), Default::default());
        }
        cache_control_config.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = CacheControlMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
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
                .middleware
                .cache_control
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = CacheControlMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
