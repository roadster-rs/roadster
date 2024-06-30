use crate::api::http::build_path;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
use axum::extract::FromRef;
use axum::routing::get;
use axum::Json;
use axum::Router;
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use tracing::instrument;

#[cfg(feature = "open-api")]
const TAG: &str = "Ping";

pub fn routes<S>(parent: &str, state: &S) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let router = Router::new();
    if !enabled(&context) {
        return router;
    }
    let root = build_path(parent, route(&context));
    router.route(&root, get(ping_get))
}

#[cfg(feature = "open-api")]
pub fn api_routes<S>(parent: &str, state: &S) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let router = ApiRouter::new();
    if !enabled(&context) {
        return router;
    }
    let root = build_path(parent, route(&context));
    router.api_route(&root, get_with(ping_get, ping_get_docs))
}

fn enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .ping
        .enabled(context)
}

fn route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .ping
        .route
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct PingResponse {}

#[instrument(skip_all)]
async fn ping_get() -> RoadsterResult<Json<PingResponse>> {
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
    use crate::app::context::AppContext;
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
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.ping.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .ping
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(super::enabled(&context), enabled);
        assert_eq!(
            super::route(&context),
            route.unwrap_or_else(|| "_ping".to_string())
        );
    }
}
