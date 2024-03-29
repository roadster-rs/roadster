// We need to use the disallowed `roadster::app_context::AppContext` type in this module in order
// to implement the required traits used to convert it to/from `AppState`.
#![allow(clippy::disallowed_types)]

use std::sync::Arc;

use roadster::app_context::AppContext;

#[derive(Debug, Clone)]
pub struct AppState {
    context: Arc<AppContext>,
}

impl AppState {
    pub fn new(ctx: Arc<AppContext>) -> Self {
        Self { context: ctx }
    }
}

impl From<Arc<AppContext>> for AppState {
    fn from(value: Arc<AppContext>) -> Self {
        AppState::new(value)
    }
}

impl From<AppState> for Arc<AppContext> {
    fn from(value: AppState) -> Self {
        value.context
    }
}
