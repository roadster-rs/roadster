#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::catch_panic::CatchPanicMiddleware;
use crate::service::http::middleware::compression::RequestDecompressionMiddleware;
use crate::service::http::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::service::http::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::service::http::middleware::size_limit::RequestBodyLimitMiddleware;
use crate::service::http::middleware::timeout::TimeoutMiddleware;
use crate::service::http::middleware::tracing::TracingMiddleware;
use crate::service::http::middleware::Middleware;
use std::collections::BTreeMap;

pub fn default_middleware<S: Send + Sync + 'static>(
    context: &AppContext<S>,
) -> BTreeMap<String, Box<dyn Middleware<S>>> {
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
    ];
    middleware
        .into_iter()
        .filter(|middleware| middleware.enabled(context))
        .map(|middleware| (middleware.name(), middleware))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(true, 9)]
    #[case(false, 0)]
    fn default_middleware(#[case] default_enable: bool, #[case] expected_size: usize) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        // Act
        let middleware = super::default_middleware(&context);

        // Assert
        assert_eq!(middleware.len(), expected_size);
    }
}
