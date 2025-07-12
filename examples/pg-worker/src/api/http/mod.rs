use aide::axum::ApiRouter;
use roadster::app::context::AppContext;

pub mod example;

pub fn routes(parent: &str) -> ApiRouter<AppContext> {
    ApiRouter::new().merge(example::routes(parent))
}
