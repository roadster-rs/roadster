use axum::extract::FromRef;
use roadster::app::context::AppContext;

#[derive(Clone)]
pub struct CustomState {
    pub context: AppContext,
    pub custom_field: String,
}

impl FromRef<CustomState> for AppContext {
    fn from_ref(input: &CustomState) -> Self {
        input.context.clone()
    }
}
