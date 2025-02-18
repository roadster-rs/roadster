use axum::extract::FromRef;
use roadster::app::context::AppContext;
use roadster::app::context::{Provide, ProvideRef};
use roadster::config::environment::Environment;
use std::ops::Deref;
use std::sync::{Arc, Weak};

#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    inner: Arc<Inner>,
}

impl Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// A version of [`AppState`] that holds a [`Weak`] pointer to the [`Inner`] state. Useful for
/// preventing reference cycles between things that are held in the [`AppState`] and also
/// need a reference to the [`AppState`]; for example, [`roadster::health_check::HealthCheck`]s.
#[derive(Clone)]
pub struct AppStateWeak {
    inner: Weak<Inner>,
}

impl AppStateWeak {
    /// Get an [`AppState`] from [`Self`].
    pub fn upgrade(&self) -> Option<AppState> {
        self.inner.upgrade().map(|inner| AppState { inner })
    }
}

#[non_exhaustive]
pub struct Inner {
    pub app_context: AppContext,
}

impl FromRef<AppState> for AppContext {
    fn from_ref(input: &AppState) -> Self {
        input.app_context.clone()
    }
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

impl AppState {
    pub fn new(app_context: AppContext) -> Self {
        Self {
            inner: Arc::new(Inner { app_context }),
        }
    }

    pub fn db(&self) -> &roadster::app::context::DieselPgPoolAsync {
        self.app_context.diesel_pg_pool_async()
    }

    pub fn is_prod(&self) -> bool {
        self.app_context.config().environment == Environment::Production
    }

    /// Get an [`AppStateWeak`] from [`Self`].
    pub fn downgrade(&self) -> AppStateWeak {
        AppStateWeak {
            inner: Arc::downgrade(&self.inner),
        }
    }
}
