use axum::extract::FromRef;
use roadster::app::context::AppContext;
use roadster::app::context::{Provide, ProvideRef};

#[derive(Clone, FromRef)]
pub struct AppState {
    pub app_context: AppContext,
}

impl<T> Provide<T> for AppState
where
    AppContext: Provide<T>,
{
    fn provide(&self) -> T {
        Provide::provide(&self.app_context)
    }
}

impl<T> ProvideRef<T> for AppState
where
    AppContext: ProvideRef<T>,
{
    fn provide(&self) -> &T {
        ProvideRef::provide(&self.app_context)
    }
}
