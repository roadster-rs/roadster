use crate::app::App;
use crate::app_context::AppContext;
use crate::service::{AppService, AppServiceBuilder};
use anyhow::bail;
use std::collections::BTreeMap;
use tracing::info;

/// Registry for [AppService]s that will be run in the app.
pub struct ServiceRegistry<A>
where
    A: App + ?Sized + 'static,
{
    pub(crate) context: AppContext<A::State>,
    pub(crate) services: BTreeMap<String, Box<dyn AppService<A>>>,
}

impl<A: App> ServiceRegistry<A> {
    pub(crate) fn new(context: &AppContext<A::State>) -> Self {
        Self {
            context: context.clone(),
            services: Default::default(),
        }
    }

    /// Register a new service. If the service is not enabled (e.g., [AppService::enabled] is `false`),
    /// the service will not be registered.
    pub fn register_service<S>(&mut self, service: S) -> anyhow::Result<()>
    where
        S: AppService<A> + 'static,
    {
        if !S::enabled(&self.context) {
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
        if !S::enabled(&self.context) || !builder.enabled(&self.context) {
            info!(service = %S::name(), "Service is not enabled, skipping building and registration");
            return Ok(());
        }

        info!(service = %S::name(), "Building service");
        let service = builder.build(&self.context).await?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
    use crate::service::{MockAppService, MockAppServiceBuilder};
    use rstest::rstest;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 0)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_service(#[case] service_enabled: bool, #[case] expected_count: usize) {
        // Arrange
        let context = AppContext::<()>::test(None, None).unwrap();

        let service: MockAppService<MockApp> = MockAppService::default();
        let enabled_ctx = MockAppService::<MockApp>::enabled_context();
        enabled_ctx.expect().returning(move |_| service_enabled);
        let name_ctx = MockAppService::<MockApp>::name_context();
        name_ctx.expect().returning(|| "test".to_string());

        // Act
        let mut subject: ServiceRegistry<MockApp> = ServiceRegistry::new(&context);
        subject.register_service(service).unwrap();

        // Assert
        assert_eq!(subject.services.len(), expected_count);
        assert_eq!(subject.services.contains_key("test"), service_enabled);
    }

    #[rstest]
    #[case(true, true, 1)]
    #[case(false, true, 0)]
    #[case(true, false, 0)]
    #[case(false, false, 0)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn register_builder(
        #[case] service_enabled: bool,
        #[case] builder_enabled: bool,
        #[case] expected_count: usize,
    ) {
        // Arrange
        let context = AppContext::<()>::test(None, None).unwrap();

        let enabled_ctx = MockAppService::<MockApp>::enabled_context();
        enabled_ctx.expect().returning(move |_| service_enabled);
        let name_ctx = MockAppService::<MockApp>::name_context();
        name_ctx.expect().returning(|| "test".to_string());

        let mut builder = MockAppServiceBuilder::default();
        builder.expect_enabled().returning(move |_| builder_enabled);
        if expected_count > 0 {
            builder
                .expect_build()
                .returning(|_| Box::pin(async move { Ok(MockAppService::default()) }));
        } else {
            builder.expect_build().never();
        }

        // Act
        let mut subject: ServiceRegistry<MockApp> = ServiceRegistry::new(&context);
        subject.register_builder(builder).await.unwrap();

        // Assert
        assert_eq!(subject.services.len(), expected_count);
        assert_eq!(subject.services.contains_key("test"), expected_count > 0);
    }
}
