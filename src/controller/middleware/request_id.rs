use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
use axum::http::HeaderName;
use axum::Router;
use std::str::FromStr;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

pub const REQUEST_ID_HEADER_NAME: &str = "request-id";

pub struct SetRequestIdMiddleware;
impl Middleware for SetRequestIdMiddleware {
    fn name(&self) -> String {
        "set-request-id".to_string()
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(SetRequestIdLayer::new(
            HeaderName::from_str(REQUEST_ID_HEADER_NAME)?,
            MakeRequestUuid,
        ));

        Ok(router)
    }
}

pub struct PropagateRequestIdMiddleware;
impl Middleware for PropagateRequestIdMiddleware {
    fn name(&self) -> String {
        "propagate-request-id".to_string()
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(PropagateRequestIdLayer::new(HeaderName::from_str(
            REQUEST_ID_HEADER_NAME,
        )?));

        Ok(router)
    }
}
