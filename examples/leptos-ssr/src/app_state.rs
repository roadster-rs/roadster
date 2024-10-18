use axum_core::extract::FromRef;
use leptos::LeptosOptions;
use leptos_config::ConfFile;
use roadster::app::context::AppContext;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub app_context: AppContext,
    pub leptos_config: ConfFile,
    pub leptos_options: LeptosOptions,
}
