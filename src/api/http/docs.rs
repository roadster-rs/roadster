use crate::api::http::build_path;
use crate::app::context::AppContext;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use axum::extract::FromRef;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Json};
use std::ops::Deref;
use std::sync::Arc;

/// This API is only available when using Aide.
pub fn routes<S>(parent: &str, state: &S) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let open_api_schema_path = build_path(parent, api_schema_route(&context));

    let router = ApiRouter::new();
    if !api_schema_enabled(&context) {
        return router;
    }

    let router = router.route(&open_api_schema_path, get(docs_get));

    let router = if scalar_enabled(&context) {
        router.route(
            &build_path(parent, scalar_route(&context)),
            get(Scalar::new(&open_api_schema_path)
                .with_title(&context.config().app.name)
                .axum_handler()),
        )
    } else {
        router
    };

    let router = if redoc_enabled(&context) {
        router.route(
            &build_path(parent, redoc_route(&context)),
            get(Redoc::new(&open_api_schema_path)
                .with_title(&context.config().app.name)
                .axum_handler()),
        )
    } else {
        router
    };

    router
}

async fn docs_get(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}

fn scalar_enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .scalar
        .enabled(context)
}

fn scalar_route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .scalar
        .route
}

fn redoc_enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .redoc
        .enabled(context)
}

fn redoc_route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .redoc
        .route
}

fn api_schema_enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .api_schema
        .enabled(context)
}

fn api_schema_route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .api_schema
        .route
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    // Todo: Is there a better way to structure these tests (and the ones in `health` and `ping`)
    //  to reduce duplication?
    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn scalar(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.scalar.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .scalar
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(scalar_enabled(&context), enabled);
        assert_eq!(
            scalar_route(&context),
            route.unwrap_or_else(|| "_docs".to_string())
        );
    }

    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn redoc(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.redoc.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .redoc
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(redoc_enabled(&context), enabled);
        assert_eq!(
            redoc_route(&context),
            route.unwrap_or_else(|| "_docs/redoc".to_string())
        );
    }

    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn api_schema(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.api_schema.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .api_schema
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(api_schema_enabled(&context), enabled);
        assert_eq!(
            api_schema_route(&context),
            route.unwrap_or_else(|| "_docs/api.json".to_string())
        );
    }
}
