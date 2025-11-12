use crate::app::context::AppContext;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
use axum::Router;
use axum_core::extract::FromRef;
use itertools::Itertools;

#[cfg(feature = "open-api")]
pub mod docs;
pub mod health;
pub mod ping;

/// This method is provided to help build API paths given a parent and child route. This is useful
/// because we recommend building your [`Router`]s and combining them using [`Router::merge`]
/// instead of [`Router::nest`] in order to allow the default-enabled
/// [`tower_http::normalize_path::NormalizePathLayer`] to work for all routes of your app --
/// otherwise, it doesn't work for nested routes.
///
/// # Examples
/// ```rust
/// # use axum::Router;
/// # use roadster::api::http::build_path;
/// # use roadster::app::context::AppContext;
/// # use roadster::service::http::builder::HttpServiceBuilder;
/// #
/// const BASE: &str = "/api";
///
/// async fn http_service(state: &AppContext) -> HttpServiceBuilder<AppContext> {
///     HttpServiceBuilder::new(Some(BASE), state)
///         .router(example_a::routes(BASE))
///         .router(example_b::routes(BASE))
/// }
///
/// mod example_a {
/// #    use axum::Router;
/// #    use axum::routing::get;
/// #    use axum_core::response::IntoResponse;
/// #    use roadster::api::http::build_path;
/// #    use roadster::app::context::AppContext;
/// #
///     pub fn routes(parent: &str) -> Router<AppContext> {
///         // Use `build_path` to build a path relative to the parent path.
///         Router::new().route(&build_path(parent, "/example_a"), get(example_a))
///     }
///
///     async fn example_a() -> impl IntoResponse {
///         ()
///     }
/// }
///
/// mod example_b {
/// #    use axum::Router;
/// #    use axum::routing::get;
/// #    use axum_core::response::IntoResponse;
/// #    use roadster::api::http::build_path;
/// #    use roadster::app::context::AppContext;
/// #
///     pub fn routes(parent: &str) -> Router<AppContext> {
///         // Use `build_path` to build a path relative to the parent path.
///         Router::new().route(&build_path(parent, "/example_b"), get(example_a))
///     }
///
///     async fn example_a() -> impl IntoResponse {
///         ()
///     }
/// }
/// ```
pub fn build_path(parent: &str, child: &str) -> String {
    // Clean the path to make sure it is valid:
    // 1. Remove any occurrences of double `/`, e.g. `/foo//bar`
    // 2. Remove any trailing `/`
    // 3. Make sure the path starts with a `/`
    let path = format!("{parent}/{child}");
    let path = path.split('/').filter(|s| !s.is_empty()).join("/");
    let path = format!("/{path}");
    path
}

pub fn default_routes<S>(parent: &str, state: &S) -> Router<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    Router::new()
        .merge(ping::routes(parent, state))
        .merge(health::routes(parent, state))
}

#[cfg(feature = "open-api")]
pub fn default_api_routes<S>(parent: &str, state: &S) -> ApiRouter<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    ApiRouter::new()
        .merge(ping::api_routes(parent, state))
        .merge(health::api_routes(parent, state))
        // The docs route is only available when using Aide
        .merge(docs::routes(parent, state))
}
