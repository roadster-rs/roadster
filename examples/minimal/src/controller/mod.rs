use crate::app_state::AppState;
use aide::axum::ApiRouter;
use roadster::app_context::AppContext;

pub mod example;

pub fn routes(parent: &str) -> ApiRouter<AppContext<AppState>> {
    ApiRouter::new().merge(example::routes(parent))
}
