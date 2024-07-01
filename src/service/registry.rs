use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::service::{AppService, AppServiceBuilder};
use anyhow::anyhow;
use axum::extract::FromRef;
use std::collections::BTreeMap;
use tracing::info;

/// Registry for [AppService]s that will be run in the app.
pub struct ServiceRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + ?Sized + 'static,
{
    pub(crate) state: S,
    pub(crate) services: BTreeMap<String, Box<dyn AppService<A, S>>>,
}

impl<A, S> ServiceRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            services: Default::default(),
        }
    }

    /// Register a new service. If the service is not enabled (e.g., [AppService::enabled] is `false`),
    /// the service will not be registered.
    pub fn register_service<Service>(&mut self, service: Service) -> RoadsterResult<()>
    where
        Service: AppService<A, S> + 'static,
    {
        self.register_internal(service)
    }

    /// Build and register a new service. If the service is not enabled (e.g.,
    /// [AppService::enabled] is `false`), the service will not be built or registered.
    pub async fn register_builder<Service, B>(&mut self, builder: B) -> RoadsterResult<()>
    where
        Service: AppService<A, S> + 'static,
        B: AppServiceBuilder<A, S, Service>,
    {
        if !builder.enabled(&self.state) {
            info!(name=%builder.name(), "Service is not enabled, skipping building and registration");
            return Ok(());
        }

        info!(name=%builder.name(), "Building service");
        let service = builder.build(&self.state).await?;

        self.register_internal(service)
    }

    fn register_internal<Service>(&mut self, service: Service) -> RoadsterResult<()>
    where
        Service: AppService<A, S> + 'static,
    {
        let name = service.name();

        if !service.enabled(&self.state) {
            info!(name=%name, "Service is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering service");

        if self
            .services
            .insert(name.clone(), Box::new(service))
            .is_some()
        {
            return Err(anyhow!("Service `{}` was already registered", name).into());
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
        let context = AppContext::test(None, None, None).unwrap();

        let mut service: MockAppService<MockApp<AppContext>, AppContext> =
            MockAppService::default();
        service.expect_enabled().return_const(service_enabled);
        service.expect_name().return_const("test".to_string());

        // Act
        let mut subject: ServiceRegistry<MockApp<AppContext>, AppContext> =
            ServiceRegistry::new(&context);
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
        let context = AppContext::test(None, None, None).unwrap();

        let mut builder = MockAppServiceBuilder::default();
        builder.expect_enabled().return_const(builder_enabled);
        builder.expect_name().return_const("test".to_string());
        builder.expect_build().returning(move |_| {
            let mut service: MockAppService<MockApp<AppContext>, AppContext> =
                MockAppService::default();
            service.expect_enabled().return_const(service_enabled);
            service.expect_name().return_const("test".to_string());
            Ok(service)
        });

        // Act
        let mut subject: ServiceRegistry<MockApp<AppContext>, AppContext> =
            ServiceRegistry::new(&context);
        subject.register_builder(builder).await.unwrap();

        // Assert
        assert_eq!(subject.services.len(), expected_count);
        assert_eq!(subject.services.contains_key("test"), expected_count > 0);
    }
}
