use crate::app_state::AppState;
use aide::axum::ApiRouter;

pub mod example;

pub fn routes(parent: &str) -> ApiRouter<AppState> {
    ApiRouter::new().merge(example::routes(parent))
}
