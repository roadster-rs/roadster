use crate::api::core::health::health_check;
use crate::api::http::build_path;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
#[cfg(any(feature = "sidekiq", feature = "db-sql"))]
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use tracing::instrument;

#[deprecated(
    since = "0.3.1",
    note = "Please import from `roadster::api::core::health` instead."
)]
pub use crate::api::core::health::{ErrorData, HeathCheckResponse, ResourceHealth, Status};

#[cfg(feature = "open-api")]
const TAG: &str = "Health";

pub fn routes<S>(parent: &str, context: &AppContext<S>) -> Router<AppContext<S>>
where
    S: Clone + Send + Sync + 'static,
{
    let router = Router::new();
    if !enabled(context) {
        return router;
    }
    let root = build_path(parent, route(context));
    router.route(&root, get(health_get::<S>))
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
    let root = build_path(parent, route(context));
    router.api_route(&root, get_with(health_get::<S>, health_get_docs))
}

fn enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .health
        .enabled(context)
}

fn route<S>(context: &AppContext<S>) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .health
        .route
}

#[instrument(skip_all)]
async fn health_get<S>(
    #[cfg(any(feature = "sidekiq", feature = "db-sql"))] State(state): State<AppContext<S>>,
) -> RoadsterResult<Json<HeathCheckResponse>>
where
    S: Clone + Send + Sync + 'static,
{
    let health = health_check::<S>(
        #[cfg(any(feature = "sidekiq", feature = "db-sql"))]
        &state,
    )
    .await?;
    Ok(Json(health))
}

#[cfg(feature = "open-api")]
fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                latency: 20,
                #[cfg(feature = "db-sql")]
                db: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: None,
                    ping_latency: None,
                    latency: 10,
                },
                #[cfg(feature = "sidekiq")]
                redis_enqueue: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(5),
                    ping_latency: Some(10),
                    latency: 15,
                },
                #[cfg(feature = "sidekiq")]
                redis_fetch: Some(ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(15),
                    ping_latency: Some(20),
                    latency: 35,
                }),
            })
        })
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    // Todo: Is there a better way to structure this test (and the ones in `docs` and `ping`)
    //  to reduce duplication?
    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn health(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.health.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .health
                .route
                .clone_from(route);
        }
        let context = AppContext::<()>::test(Some(config), None, None).unwrap();

        assert_eq!(super::enabled(&context), enabled);
        assert_eq!(
            super::route(&context),
            route.unwrap_or_else(|| "_health".to_string())
        );
    }
}
