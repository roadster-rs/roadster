use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::lifecycle::default::default_lifecycle_handlers;
use crate::lifecycle::AppLifecycleHandler;
use anyhow::anyhow;
use axum::extract::FromRef;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::ops::Deref;
use tracing::info;

/// Registry for the app's [`AppLifecycleHandler`]s.
pub struct LifecycleHandlerRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + ?Sized + 'static,
{
    state: S,
    handlers: BTreeMap<String, Box<dyn AppLifecycleHandler<A, S>>>,
}

impl<A, S> LifecycleHandlerRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            handlers: default_lifecycle_handlers(state),
        }
    }

    /// Register a new [`AppLifecycleHandler`]. If the [`AppLifecycleHandler`] is not enabled
    /// (e.g., [[`AppLifecycleHandler::enabled`] returns `false`), the [`AppLifecycleHandler`]
    /// will not be registered.
    pub fn register<H>(&mut self, handler: H) -> RoadsterResult<()>
    where
        H: AppLifecycleHandler<A, S> + 'static,
    {
        self.register_boxed(Box::new(handler))
    }

    pub(crate) fn register_boxed(
        &mut self,
        handler: Box<dyn AppLifecycleHandler<A, S>>,
    ) -> RoadsterResult<()> {
        let name = handler.name();

        if !handler.enabled(&self.state) {
            info!(name=%name, "Lifecycle handler is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering lifecycle handler");

        if self.handlers.insert(name.clone(), handler).is_some() {
            return Err(anyhow!("Handler `{}` was already registered", name).into());
        }

        Ok(())
    }

    /// Get the registered [`AppLifecycleHandler`]s, ordered by their
    /// [`AppLifecycleHandler::priority`].
    pub(crate) fn handlers(&self, state: &S) -> Vec<&dyn AppLifecycleHandler<A, S>> {
        self.handlers
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            .map(|handler| handler.deref())
            .collect_vec()
    }
}
