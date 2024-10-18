use axum_core::extract::FromRef;
use roadster::app::context::AppContext;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub app_context: AppContext,
}
