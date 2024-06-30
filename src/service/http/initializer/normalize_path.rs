use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::initializer::Initializer;
use axum::extract::FromRef;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct NormalizePathConfig {}

pub struct NormalizePathInitializer;

impl<S> Initializer<S> for NormalizePathInitializer
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
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

    fn before_serve(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let router = Router::new().nest_service("/", router);
        Ok(router)
    }
}
