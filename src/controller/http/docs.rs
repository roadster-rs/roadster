#[mockall_double::double]
use crate::app_context::AppContext;
use std::ops::Deref;
use std::sync::Arc;

use crate::config::app_config::AppConfig;
use crate::controller::http::build_path;
use aide::axum::routing::get_with;
use aide::axum::{ApiRouter, IntoApiResponse};
use aide::openapi::OpenApi;
use aide::redoc::Redoc;
use aide::scalar::Scalar;
use axum::response::IntoResponse;
use axum::{Extension, Json};

const BASE: &str = "_docs";
const TAG: &str = "Docs";

/// This API is only available when using Aide.
pub fn routes<S>(parent: &str, context: &AppContext<S>) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let parent = build_path(parent, BASE);
    let open_api_schema_path = build_path(&parent, &api_schema_route(context));

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
            &build_path(&parent, &scalar_route(context)),
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
            &build_path(&parent, &redoc_route(context)),
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

fn scalar_route<S>(context: &AppContext<S>) -> String {
    let config: &AppConfig = context.config();
    config
        .service
        .http
        .custom
        .default_routes
        .scalar
        .route
        .clone()
        .unwrap_or_else(|| "/".to_string())
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

fn redoc_route<S>(context: &AppContext<S>) -> String {
    let config: &AppConfig = context.config();
    config
        .service
        .http
        .custom
        .default_routes
        .redoc
        .route
        .clone()
        .unwrap_or_else(|| "redoc".to_string())
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

fn api_schema_route<S>(context: &AppContext<S>) -> String {
    let config: &AppConfig = context.config();
    config
        .service
        .http
        .custom
        .default_routes
        .api_schema
        .route
        .clone()
        .unwrap_or_else(|| "api.json".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
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
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.scalar.enable = enable;
        config
            .service
            .http
            .custom
            .default_routes
            .scalar
            .route
            .clone_from(&route);
        let mut context = MockAppContext::<MockApp>::default();
        context.expect_config().return_const(config);

        assert_eq!(scalar_enabled(&context), enabled);
        assert_eq!(
            scalar_route(&context),
            route.unwrap_or_else(|| "/".to_string())
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
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.redoc.enable = enable;
        config
            .service
            .http
            .custom
            .default_routes
            .redoc
            .route
            .clone_from(&route);
        let mut context = MockAppContext::<MockApp>::default();
        context.expect_config().return_const(config);

        assert_eq!(redoc_enabled(&context), enabled);
        assert_eq!(
            redoc_route(&context),
            route.unwrap_or_else(|| "redoc".to_string())
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
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.api_schema.enable = enable;
        config
            .service
            .http
            .custom
            .default_routes
            .api_schema
            .route
            .clone_from(&route);
        let mut context = MockAppContext::<MockApp>::default();
        context.expect_config().return_const(config);

        assert_eq!(api_schema_enabled(&context), enabled);
        assert_eq!(
            api_schema_route(&context),
            route.unwrap_or_else(|| "api.json".to_string())
        );
    }
}
