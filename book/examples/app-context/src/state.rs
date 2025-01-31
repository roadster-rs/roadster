use axum::extract::FromRef;
use roadster::app::context::AppContext;

#[derive(Clone, FromRef)]
pub struct CustomState {
    pub context: AppContext,
    pub custom_field: String,
}
