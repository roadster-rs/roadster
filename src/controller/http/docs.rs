#[mockall_double::double]
use crate::app_context::AppContext;
use std::ops::Deref;
use std::sync::Arc;

use crate::controller::http::build_path;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::{Extension, Json};

const TAG: &str = "Docs";

/// This API is only available when using Aide.
pub fn routes<S>(parent: &str, context: &AppContext<S>) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    if scalar_enabled(context) && !api_schema_enabled(context) {
        debug_assert!(
            false,
            "The Open API schema route must be enabled in order to use the Scalar docs route."
        );
    }
    if redoc_enabled(context) && !api_schema_enabled(context) {
        debug_assert!(
            false,
            "The Open API schema route must be enabled in order to use the Redoc docs route."
        );
    }

    let open_api_schema_path = build_path(parent, api_schema_route(context));

    let router = ApiRouter::new();
    if !api_schema_enabled(context) {
        return router;
    }

    let router = router.api_route(
        &open_api_schema_path,
        get_with(docs_get, |op| op.description("OpenAPI schema").tag(TAG)),
    );

    let router = if scalar_enabled(context) {
        router.api_route_with(
            &build_path(parent, scalar_route(context)),
            get_with(
                Scalar::new(&open_api_schema_path)
                    .with_title(&context.config().app.name)
                    .axum_handler(),
                |op| op.description("Documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
    } else {
        router
    };

    let router = if redoc_enabled(context) {
        router.api_route_with(
            &build_path(parent, redoc_route(context)),
            get_with(
                Redoc::new(&open_api_schema_path)
                    .with_title(&context.config().app.name)
                    .axum_handler(),
                |op| op.description("Redoc documentation page.").tag(TAG),
            ),
            |p| p.security_requirement("ApiKey"),
        )
    } else {
        router
    };

    router
}

async fn docs_get(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}

fn scalar_enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .scalar
        .enabled(context)
}

fn scalar_route<S>(context: &AppContext<S>) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .scalar
        .route
}

fn redoc_enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .redoc
        .enabled(context)
}

fn redoc_route<S>(context: &AppContext<S>) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .redoc
        .route
}

fn api_schema_enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .api_schema
        .enabled(context)
}

fn api_schema_route<S>(context: &AppContext<S>) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .api_schema
        .route
}
