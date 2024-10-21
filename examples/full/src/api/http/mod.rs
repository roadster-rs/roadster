use crate::app_state::AppState;
use aide::axum::ApiRouter;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::info;

pub mod example;

pub fn routes(parent: &str) -> ApiRouter<AppState> {
    ApiRouter::new().merge(example::routes(parent))
}

pub(crate) async fn hello_world_middleware_fn(request: Request, next: Next) -> Response {
    info!("Running `hello-world` middleware");

    next.run(request).await
}
