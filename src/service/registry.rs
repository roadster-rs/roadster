use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::service::{AppService, AppServiceBuilder};
use axum_core::extract::FromRef;
use std::any::{type_name, TypeId};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceRegistryError {
    /// The provided [`AppService`] was already registered. Contains the [`AppService::name`]
    /// of the provided service.
    #[error("The provided `AppService` was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// Unable to find an [`AppService`] instance of the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to find an `AppService` instance of type `{0}`")]
    NotRegistered(String),

    /// Unable to downcast the registered instance to the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to downcast the registered instance of `AppService` to type `{0}`")]
    Downcast(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Registry for [AppService]s that will be run in the app.
pub struct ServiceRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    pub(crate) state: S,
    pub(crate) service_names: HashSet<String>,
    pub(crate) services: BTreeMap<TypeId, Box<dyn AppService<A, S>>>,
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
            service_names: Default::default(),
            services: Default::default(),
        }
    }

    /// Register a new service. If the service is not enabled (e.g., [AppService::enabled] is `false`),
    /// the service will not be registered.
    pub fn register_service<Service>(&mut self, service: Service) -> RoadsterResult<()>
    where
        Service: AppService<A, S> + 'static,
    {
        self.register_boxed(Box::new(service))
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

        self.register_boxed(Box::new(service))
    }

    pub(crate) fn register_boxed(
        &mut self,
        service: Box<dyn AppService<A, S>>,
    ) -> RoadsterResult<()> {
        let name = service.name();

        if !service.enabled(&self.state) {
            info!(name=%name, "Service is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering service");

        if !self.service_names.insert(name.clone())
            || self
                .services
                .insert(service.as_any().type_id(), service)
                .is_some()
        {
            return Err(ServiceRegistryError::AlreadyRegistered(name.clone()).into());
        }
        Ok(())
    }

    /// Get a reference to a previously registered [`AppService`] of the specified type.
    ///
    /// This is useful to call a method that only exists on a concrete [`AppService`]
    /// implementor after the app was prepared. For example, to get the OpenAPI schema for an app,
    /// setup and register the [`crate::service::http::service::HttpService`], get the service
    /// from the registry with this method ([`ServiceRegistry::get`]), and call
    /// [`crate::service::http::service::HttpService::print_open_api_schema`] to get the schema.
    ///
    /// # Examples
    #[cfg_attr(
        feature = "open-api",
        doc = r##"
  ```rust
# tokio_test::block_on(async {
# use roadster::service::http::service::OpenApiArgs;
# use roadster::app::RoadsterApp;
# use roadster::util::empty::Empty;
# use roadster::service::AppServiceBuilder;
# use roadster::service::http::service::HttpService;
# use std::env::current_dir;
# use std::path::PathBuf;
# use std::sync::LazyLock;
# use uuid::Uuid;
# use roadster::app::PrepareOptions;
# use roadster::config::environment::Environment;
# use async_trait::async_trait;
# use tokio_util::sync::CancellationToken;
# use roadster::app::context::AppContext;
# use roadster::error::RoadsterResult;
# use roadster::service::function::service::FunctionService;
# use roadster::service::registry::ServiceRegistry;
# use roadster::app::prepare;
# use roadster::service::AppService;
#
type App = RoadsterApp<AppContext, Empty>;

let app: App = RoadsterApp::builder()
    .state_provider(|state| Ok(state))
    .add_service_provider(|registry, state| Box::pin(async  {
        registry.register_builder(
            HttpService::builder(Some("/api"), state)
        ).await?;
        Ok(())
    }))
    .build();

// Prepare the app. This runs all initialization logic for the app but does not actually
// start the app.
let prepared = prepare(
    app,
    PrepareOptions::builder()
        .env(Environment::Development)
#       .config_dir(PathBuf::from("examples/full/config").canonicalize().unwrap())
        .build()
).await.unwrap();
// Get the `HttpService` from the `ServiceRegistry`
let http_service = prepared.service_registry.get::<HttpService>().unwrap();
// Get the OpenAPI schema from the `HttpService`
http_service.open_api_schema(&OpenApiArgs::builder().build()).unwrap();
# })
```
"##
    )]
    pub fn get<Service>(&self) -> RoadsterResult<&Service>
    where
        Service: AppService<A, S> + 'static,
    {
        let service = self
            .services
            .get(&TypeId::of::<Service>())
            .ok_or_else(|| ServiceRegistryError::NotRegistered(type_name::<Service>().to_string()))?
            .as_any()
            .downcast_ref::<Service>()
            .ok_or_else(|| ServiceRegistryError::Downcast(type_name::<Service>().to_string()))?;
        Ok(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
    use crate::error::Error;
    use crate::service::{MockAppService, MockAppServiceBuilder};
    use async_trait::async_trait;
    use rstest::rstest;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

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
        assert_eq!(subject.services.len(), subject.service_names.len());
        assert_eq!(
            subject
                .services
                .contains_key(&TypeId::of::<MockAppService<MockApp<AppContext>, AppContext>>()),
            service_enabled
        );
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
        assert_eq!(subject.services.len(), subject.service_names.len());
        assert_eq!(
            subject
                .services
                .contains_key(&TypeId::of::<MockAppService<MockApp<AppContext>, AppContext>>()),
            expected_count > 0
        );
    }

    struct FooService {
        id: Uuid,
    }
    #[async_trait]
    #[cfg_attr(coverage_nightly, coverage(off))]
    impl AppService<MockApp<AppContext>, AppContext> for FooService {
        fn name(&self) -> String {
            "foo".to_string()
        }
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn enabled(&self, _: &AppContext) -> bool {
            true
        }
        #[cfg_attr(coverage_nightly, coverage(off))]
        async fn run(self: Box<Self>, _: &AppContext, _: CancellationToken) -> RoadsterResult<()> {
            todo!()
        }
    }

    struct BarService;
    #[async_trait]
    #[cfg_attr(coverage_nightly, coverage(off))]
    impl AppService<MockApp<AppContext>, AppContext> for BarService {
        fn name(&self) -> String {
            "bar".to_string()
        }
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn enabled(&self, _: &AppContext) -> bool {
            true
        }
        #[cfg_attr(coverage_nightly, coverage(off))]
        async fn run(self: Box<Self>, _: &AppContext, _: CancellationToken) -> RoadsterResult<()> {
            todo!()
        }
    }

    #[rstest]
    #[case(true, true)]
    #[case(false, true)]
    #[case(false, false)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn get(#[case] registered: bool, #[case] correct_type: bool) {
        // Arrange
        let context = AppContext::test(None, None, None).unwrap();

        let id = Uuid::new_v4();
        let service = FooService { id };

        let mut subject: ServiceRegistry<MockApp<AppContext>, AppContext> =
            ServiceRegistry::new(&context);
        if registered && correct_type {
            subject.register_service(service).unwrap();

            let duplicate = subject.register_service(FooService { id: Uuid::new_v4() });
            assert!(matches!(
                duplicate,
                Err(Error::ServiceRegistry(
                    ServiceRegistryError::AlreadyRegistered(_)
                ))
            ));
        } else if registered && !correct_type {
            subject.register_service(BarService).unwrap();
        }

        // Act
        let service = subject.get::<FooService>();

        if !registered {
            assert!(matches!(
                service,
                Err(Error::ServiceRegistry(ServiceRegistryError::NotRegistered(
                    _
                )))
            ));
        } else if !correct_type {
            assert!(matches!(
                service,
                Err(Error::ServiceRegistry(ServiceRegistryError::Downcast(_)))
            ));
        } else {
            assert_eq!(service.unwrap().id, id);
        }
    }
}
