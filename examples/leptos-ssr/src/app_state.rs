use axum::extract::FromRef;
use leptos::prelude::*;
use roadster::app::context::AppContext;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub app_context: AppContext,
    pub leptos_config: ConfFile,
    pub leptos_options: LeptosOptions,
}
