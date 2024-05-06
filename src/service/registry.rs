use crate::app::App;
use crate::app_context::AppContext;
use crate::service::{AppService, AppServiceBuilder};
use anyhow::bail;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::info;

/// Registry for [AppService]s that will be run in the app.
pub struct ServiceRegistry<A>
where
    A: App + ?Sized,
{
    pub(crate) context: Arc<AppContext>,
    pub(crate) state: Arc<A::State>,
    pub(crate) services: BTreeMap<String, Box<dyn AppService<A>>>,
}

impl<A: App> ServiceRegistry<A> {
    pub(crate) fn new(context: Arc<AppContext>, state: Arc<A::State>) -> Self {
        Self {
            context,
            state,
            services: Default::default(),
        }
    }

    /// Register a new service. If the service is not enabled (e.g., [AppService::enabled] is `false`),
    /// the service will not be registered.
    pub fn register_service<S>(&mut self, service: S) -> anyhow::Result<()>
    where
        S: AppService<A> + 'static,
    {
        if !S::enabled(&self.context, &self.state) {
            info!(service = %S::name(), "Service is not enabled, skipping registration");
            return Ok(());
        }
        self.register_internal(service)
    }

    /// Build and register a new service. If the service is not enabled (e.g.,
    /// [AppService::enabled] is `false`), the service will not be built or registered.
    pub async fn register_builder<S, B>(&mut self, builder: B) -> anyhow::Result<()>
    where
        S: AppService<A> + 'static,
        B: AppServiceBuilder<A, S>,
    {
        if !S::enabled(&self.context, &self.state) || !builder.enabled(&self.context, &self.state) {
            info!(service = %S::name(), "Service is not enabled, skipping building and registration");
            return Ok(());
        }

        info!(service = %S::name(), "Building service");
        let service = builder.build(&self.context, &self.state).await?;

        self.register_internal(service)
    }

    fn register_internal<S>(&mut self, service: S) -> anyhow::Result<()>
    where
        S: AppService<A> + 'static,
    {
        info!(service = %S::name(), "Registering service");

        if self.services.insert(S::name(), Box::new(service)).is_some() {
            bail!("Service `{}` was already registered", S::name());
        }
        Ok(())
    }
}
