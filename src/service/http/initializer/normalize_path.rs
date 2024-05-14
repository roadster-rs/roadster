#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::initializer::Initializer;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct NormalizePathConfig {}

pub struct NormalizePathInitializer;

impl<S: Send + Sync + 'static> Initializer<S> for NormalizePathInitializer {
    fn name(&self) -> String {
        "normalize-path".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .initializer
            .normalize_path
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext<S>) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .initializer
            .normalize_path
            .common
            .priority
    }

    fn before_serve(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let router = Router::new().nest_service("/", router);
        Ok(router)
    }
}
