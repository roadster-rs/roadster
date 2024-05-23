#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::app_config::AppConfig;
use crate::controller::http::build_path;
use crate::view::http::app_error::AppError;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
use axum::routing::get;
use axum::Json;
use axum::Router;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

#[cfg(feature = "open-api")]
const TAG: &str = "Ping";

pub fn routes<S>(parent: &str, context: &AppContext<S>) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new();
    if !enabled(context) {
        return router;
    }
    let root = build_path(parent, &route(context));
    router.route(&root, get(ping_get))
}

#[cfg(feature = "open-api")]
pub fn api_routes<S>(parent: &str, context: &AppContext<S>) -> ApiRouter<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let router = ApiRouter::new();
    if !enabled(context) {
        return router;
    }
    let root = build_path(parent, &route(context));
    router.api_route(&root, get_with(ping_get, ping_get_docs))
}

fn enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .ping
        .enabled(context)
}

fn route<S>(context: &AppContext<S>) -> String {
    let config: &AppConfig = context.config();
    config
        .service
        .http
        .custom
        .default_routes
        .ping
        .route
        .clone()
        .unwrap_or_else(|| "_ping".to_string())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {}

#[instrument(skip_all)]
async fn ping_get() -> Result<Json<PingResponse>, AppError> {
    Ok(Json(PingResponse::default()))
}

#[cfg(feature = "open-api")]
fn ping_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Ping the server to confirm that it is running.")
        .tag(TAG)
        .response_with::<200, Json<PingResponse>, _>(|res| res.example(PingResponse::default()))
}

#[cfg(test)]
mod tests {
    use crate::app::MockApp;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    // Todo: Is there a better way to structure this test (and the ones in `health` and `ping`)
    //  to reduce duplication?
    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn ping(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.ping.enable = enable;
        config
            .service
            .http
            .custom
            .default_routes
            .ping
            .route
            .clone_from(&route);
        let mut context = MockAppContext::<MockApp>::default();
        context.expect_config().return_const(config);

        assert_eq!(super::enabled(&context), enabled);
        assert_eq!(
            super::route(&context),
            route.unwrap_or_else(|| "_ping".to_string())
        );
    }
}
