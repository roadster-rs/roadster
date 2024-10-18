use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::initializer::Initializer;
use axum::Router;
use axum_core::extract::FromRef;
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
    fn before_serve(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        let router = NormalizePathLayer::trim_trailing_slash().layer(router);
        let router = Router::new().nest_service("/", router);
        Ok(router)
    }
}
