use crate::api::http::build_path;
use crate::app::context::AppContext;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use aide::swagger::Swagger;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_core::extract::FromRef;
use std::ops::Deref;
use std::sync::Arc;

const TAG: &str = "Docs";

/// This API is only available when using Aide.
pub fn routes<S>(state: &S, parent: &str) -> ApiRouter<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let open_api_schema_path = build_path(parent, api_schema_route(&context));

    let router = ApiRouter::new();
    if !api_schema_enabled(&context) {
        return router;
    }

    let router = router.api_route(
        &open_api_schema_path,
        get_with(docs_get, |op| {
            op.tag(TAG).description("The OpenAPI schema as JSON")
        }),
    );

    let router = if swagger_enabled(&context) {
        router.api_route(
            &build_path(parent, swagger_route(&context)),
            get_with(
                Swagger::new(&open_api_schema_path)
                    .with_title(&context.config().app.name)
                    .axum_handler(),
                |op| op.tag(TAG).description("Swagger UI API explorer"),
            ),
        )
    } else {
        router
    };

    let router = if scalar_enabled(&context) {
        router.api_route(
            &build_path(parent, scalar_route(&context)),
            get_with(
                Scalar::new(&open_api_schema_path)
                    .with_title(&context.config().app.name)
                    .axum_handler(),
                |op| op.tag(TAG).description("Scalar OpenAPI explorer"),
            ),
        )
    } else {
        router
    };

    if redoc_enabled(&context) {
        router.api_route(
            &build_path(parent, redoc_route(&context)),
            get_with(
                Redoc::new(&open_api_schema_path)
                    .with_title(&context.config().app.name)
                    .axum_handler(),
                |op| op.tag(TAG).description("Redoc OpenAPI explorer"),
            ),
        )
    } else {
        router
    }
}

async fn docs_get(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.deref()).into_response()
}

fn swagger_enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .swagger
        .enabled(context)
}

fn swagger_route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .swagger
        .route
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
    fn swagger(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.swagger.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .swagger
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(swagger_enabled(&context), enabled);
        assert_eq!(
            swagger_route(&context),
            route.unwrap_or_else(|| "_docs".to_string())
        );
    }

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
            route.unwrap_or_else(|| "_docs/swagger".to_string())
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
