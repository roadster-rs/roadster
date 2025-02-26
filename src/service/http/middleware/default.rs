use crate::app::context::AppContext;
use crate::service::http::middleware::Middleware;
use crate::service::http::middleware::cache_control::CacheControlMiddleware;
use crate::service::http::middleware::catch_panic::CatchPanicMiddleware;
use crate::service::http::middleware::compression::RequestDecompressionMiddleware;
use crate::service::http::middleware::cors::CorsMiddleware;
use crate::service::http::middleware::etag::EtagMiddleware;
use crate::service::http::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::service::http::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::service::http::middleware::size_limit::RequestBodyLimitMiddleware;
use crate::service::http::middleware::timeout::TimeoutMiddleware;
use crate::service::http::middleware::tracing::TracingMiddleware;
use crate::service::http::middleware::tracing::req_res_logging::RequestResponseLoggingMiddleware;
use axum_core::extract::FromRef;
use std::collections::BTreeMap;

pub fn default_middleware<S>(state: &S) -> BTreeMap<String, Box<dyn Middleware<S>>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let middleware: Vec<Box<dyn Middleware<S>>> = vec![
        Box::new(SensitiveRequestHeadersMiddleware),
        Box::new(SensitiveResponseHeadersMiddleware),
        Box::new(SetRequestIdMiddleware),
        Box::new(PropagateRequestIdMiddleware),
        Box::new(TracingMiddleware),
        Box::new(CatchPanicMiddleware),
        Box::new(RequestDecompressionMiddleware),
        Box::new(TimeoutMiddleware),
        Box::new(RequestBodyLimitMiddleware),
        Box::new(CorsMiddleware),
        Box::new(RequestResponseLoggingMiddleware),
        Box::new(CacheControlMiddleware),
        Box::new(EtagMiddleware),
    ];

    middleware
        .into_iter()
        .filter(|middleware| middleware.enabled(state))
        .map(|middleware| (middleware.name(), middleware))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_middleware(_case: TestCase, #[case] default_enable: bool) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;

        config
            .service
            .http
            .custom
            .middleware
            .cache_control
            .custom
            .content_types
            .insert("text/css".to_string(), Default::default());

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act
        let middleware = super::default_middleware(&context);
        let middleware = middleware.keys().collect_vec();

        // Assert
        assert_toml_snapshot!(middleware);
    }
}
