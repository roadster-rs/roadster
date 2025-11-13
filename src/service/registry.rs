use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::{Service, ServiceBuilder};
use axum_core::extract::FromRef;
use std::any::{Any, TypeId, type_name};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServiceRegistryError {
    /// The provided [`Service`] was already registered. Contains the [`Service::name`]
    /// of the provided service.
    #[error("The provided `Service` was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// Unable to find a [`Service`] instance of the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to find an `Service` instance of type `{0}`")]
    NotRegistered(String),

    /// The [`Service`] was already ran using [`Service::run`], so it is no longer available
    /// in the [`ServiceRegistry`].
    #[error("`Service` instance of type `{0}` was already ran")]
    AlreadyRan(String),

    /// Unable to downcast the registered instance to the requested type. Contains the [`type_name`]
    /// of the requested type.
    #[error("Unable to downcast the registered instance of `Service` to type `{0}`")]
    Downcast(String),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

/// Registry for [`Service`]s that will be run in the app.
pub struct ServiceRegistry<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) state: S,
    pub(crate) service_names: HashSet<String>,
    pub(crate) services: BTreeMap<TypeId, ServiceWrapper<S>>,
}

impl<S> ServiceRegistry<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            service_names: Default::default(),
            services: Default::default(),
        }
    }

    /// Register a new service. If the service is not enabled (e.g., [`Service::enabled`] is `false`),
    /// the service will not be registered.
    pub fn register_service<Srvc>(&mut self, service: Srvc) -> RoadsterResult<()>
    where
        Srvc: 'static + Service<S>,
    {
        self.register_wrapped(ServiceWrapper::new(service))
    }

    /// Build and register a new service. If the service is not enabled (e.g.,
    /// [`Service::enabled`] is `false`), the service will not be built or registered.
    pub async fn register_builder<Srvc, B>(&mut self, builder: B) -> RoadsterResult<()>
    where
        Srvc: 'static + Service<S>,
        B: 'static + ServiceBuilder<S, Srvc>,
    {
        if !builder.enabled(&self.state) {
            info!(service.builder.name=%builder.name(), "Service is not enabled, skipping building and registration");
            return Ok(());
        }

        info!(service.builder.name=%builder.name(), "Building service");
        let service = builder
            .build(&self.state)
            .await
            .map_err(|err| ServiceRegistryError::Other(Box::new(err)))?;

        self.register_wrapped(ServiceWrapper::new(service))
    }

    pub(crate) fn register_wrapped(&mut self, service: ServiceWrapper<S>) -> RoadsterResult<()> {
        let name = service.name();

        info!(service.name=%name, "Registering service");

        if !self.service_names.insert(name.clone())
            || self.services.insert(service.type_id, service).is_some()
        {
            return Err(ServiceRegistryError::AlreadyRegistered(name).into());
        }
        Ok(())
    }

    /// Invoke a callback on a reference to a previously registered [`Service`] of the specified
    /// type.
    ///
    /// This is useful to call a method that only exists on a concrete [`Service`]
    /// implementor after the app was prepared.
    #[cfg_attr(
        all(feature = "http", feature = "open-api"),
        doc = r##"
For example, to get the OpenAPI schema for an app,
setup and register the [`crate::service::http::service::HttpService`], get the service
from the registry with this method ([`ServiceRegistry::invoke`]), and call
[`crate::service::http::service::HttpService::print_open_api_schema`] to get the schema.
    "##
    )]
    ///
    /// # Examples
    #[cfg_attr(
        all(feature = "open-api", feature = "otel-grpc"),
        doc = r##"
  ```rust
# tokio_test::block_on(async {
# use roadster::service::http::service::OpenApiArgs;
# use roadster::app::RoadsterApp;
# use roadster::service::ServiceBuilder;
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
# use roadster::service::Service;
#
type App = RoadsterApp<AppContext>;

let app: App = RoadsterApp::builder()
    .state_provider(|state| Ok(state))
    .add_service_provider(|registry, state| Box::pin(async  {
        registry.register_builder(
            HttpService::builder(state, Some("/api"))
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
// Get the `HttpService` from the `ServiceRegistry` and get the OpenAPI schema.
prepared.service_registry.invoke(async |service: &HttpService| {
    service.open_api_schema(&OpenApiArgs::builder().build()).unwrap();
}).await;
# })
```
"##
    )]
    pub async fn invoke<Srvc, F, R>(&self, invoke: F) -> RoadsterResult<R>
    where
        Srvc: 'static + Service<S>,
        F: AsyncFnOnce(&Srvc) -> R,
    {
        let service_wrapper = self
            .services
            .get(&TypeId::of::<Srvc>())
            .ok_or_else(|| ServiceRegistryError::NotRegistered(type_name::<Srvc>().to_string()))?;
        let guard = service_wrapper.inner.lock().await;
        let inner = guard
            .as_ref()
            .ok_or_else(|| ServiceRegistryError::AlreadyRan(type_name::<Srvc>().to_string()))?;
        let srvc = inner
            .downcast_ref::<Srvc>()
            .ok_or_else(|| ServiceRegistryError::Downcast(type_name::<Srvc>().to_string()))?;
        let result = invoke(srvc).await;
        Ok(result)
    }
}

type EnabledFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S) -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = bool>>>,
>;

type BeforeRunFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S) -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

type RunFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            CancellationToken,
        )
            -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

/// Wrapper around a [`Service`] to allow storing the [`Service`]s in a collection regardless of
/// their [`Service::Error`] associated types.
pub(crate) struct ServiceWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type_id: TypeId,
    name: String,
    enabled_fn: EnabledFn<S>,
    before_run_fn: BeforeRunFn<S>,
    run_fn: RunFn<S>,
    inner: Arc<Mutex<Option<Box<dyn Send + Sync + Any>>>>,
}

impl<S> ServiceWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new<Srvc>(service: Srvc) -> Self
    where
        Srvc: 'static + Send + Sync + Any + Service<S>,
    {
        let type_id = service.type_id();
        let name = service.name();
        /*
        For some reason, we need to explicitly annotate the type here. If we don't, Rust
        complains about the value inside the `Box` not being compatible with the `Send + Sync + Any`
        trait bounds when trying to assign the value to the `ServiceWrapper#inner` field. This is
        also why we need to downcast in the method wrappers below (we don't have a handle to a
        `Service` instance, just an `Any`.
         */
        let inner: Arc<Mutex<Option<Box<dyn Send + Sync + Any>>>> =
            Arc::new(Mutex::new(Some(Box::new(service))));
        let enabled_fn: EnabledFn<S> = {
            let inner = inner.clone();
            Box::new(move |state| {
                let inner = inner.clone();
                Box::pin(async move {
                    let guard = inner.lock().await;
                    #[allow(clippy::expect_used)]
                    let inner = guard
                        .as_ref()
                        .unwrap_or_else(|| panic!("`Service#enabled` can not be called for `Service` of type `{}`; `Service#run` was already called", type_name::<Srvc>()))
                        .downcast_ref::<Srvc>()
                        .unwrap_or_else(|| panic!("Registered `Service` can not be downcast to type `{}`", type_name::<Srvc>()));
                    inner.enabled(state)
                })
            })
        };
        let before_run_fn: BeforeRunFn<S> = {
            let inner = inner.clone();
            Box::new(move |state| {
                let inner = inner.clone();
                Box::pin(async move {
                    let guard = inner.lock().await;
                    let inner = guard
                        .as_ref()
                        .ok_or_else(|| {
                            ServiceRegistryError::AlreadyRan(type_name::<Srvc>().to_string())
                        })?
                        .downcast_ref::<Srvc>()
                        .ok_or_else(|| {
                            ServiceRegistryError::Downcast(type_name::<Srvc>().to_string())
                        })?;
                    inner
                        .before_run(state)
                        .await
                        .map_err(|err| ServiceRegistryError::Other(Box::new(err)))?;
                    Ok(())
                })
            })
        };
        let run_fn: RunFn<S> = {
            let inner = inner.clone();
            Box::new(move |state, cancellation_token| {
                let inner = inner.clone();
                Box::pin(async move {
                    let mut guard = inner.lock().await;
                    let inner = guard
                        .take()
                        .ok_or_else(|| {
                            ServiceRegistryError::AlreadyRan(type_name::<Srvc>().to_string())
                        })?
                        .downcast::<Srvc>()
                        .map_err(|_err| {
                            ServiceRegistryError::Downcast(type_name::<Srvc>().to_string())
                        })?;
                    inner
                        .run(state, cancellation_token)
                        .await
                        .map_err(|err| ServiceRegistryError::Other(Box::new(err)))?;
                    Ok(())
                })
            })
        };
        Self {
            type_id,
            name,
            enabled_fn,
            before_run_fn,
            run_fn,
            inner,
        }
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    pub(crate) async fn enabled(&self, state: &S) -> bool {
        (self.enabled_fn)(state).await
    }

    pub(crate) async fn before_run(&self, state: &S) -> RoadsterResult<()> {
        (self.before_run_fn)(state).await
    }

    pub(crate) async fn run(
        &self,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        (self.run_fn)(state, cancel_token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::service::{MockService, MockServiceBuilder};
    use async_trait::async_trait;
    use rstest::rstest;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 1)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_service(#[case] service_enabled: bool, #[case] expected_count: usize) {
        // Arrange
        let context = AppContext::test(None, None, None).unwrap();

        let mut service: MockService<AppContext> = MockService::default();
        service.expect_enabled().return_const(service_enabled);
        service.expect_name().return_const("test".to_string());

        // Act
        let mut subject: ServiceRegistry<AppContext> = ServiceRegistry::new(&context);
        subject.register_service(service).unwrap();

        // Assert
        assert_eq!(subject.services.len(), expected_count);
        assert_eq!(subject.services.len(), subject.service_names.len());
        assert!(
            subject
                .services
                .contains_key(&TypeId::of::<MockService<AppContext>>())
        );
    }

    #[rstest]
    #[case(true, true, 1)]
    #[case(false, true, 1)]
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

        let mut builder = MockServiceBuilder::default();
        builder.expect_enabled().return_const(builder_enabled);
        builder.expect_name().return_const("test".to_string());
        builder.expect_build().returning(move |_| {
            let mut service: MockService<AppContext> = MockService::default();
            service.expect_enabled().return_const(service_enabled);
            service.expect_name().return_const("test".to_string());
            Ok(service)
        });

        // Act
        let mut subject: ServiceRegistry<AppContext> = ServiceRegistry::new(&context);
        subject.register_builder(builder).await.unwrap();

        // Assert
        assert_eq!(subject.services.len(), expected_count);
        assert_eq!(subject.services.len(), subject.service_names.len());
        assert_eq!(
            subject
                .services
                .contains_key(&TypeId::of::<MockService<AppContext>>()),
            expected_count > 0
        );
    }

    struct FooService {
        id: Uuid,
    }
    #[async_trait]
    #[cfg_attr(coverage_nightly, coverage(off))]
    impl Service<AppContext> for FooService {
        type Error = crate::error::Error;

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
    impl Service<AppContext> for BarService {
        type Error = crate::error::Error;

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
    async fn invoke(#[case] registered: bool, #[case] correct_type: bool) {
        // Arrange
        let context = AppContext::test(None, None, None).unwrap();

        let id = Uuid::new_v4();
        let service = FooService { id };

        let mut subject: ServiceRegistry<AppContext> = ServiceRegistry::new(&context);
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
        let service = subject
            .invoke::<FooService, _, _>(async |srvc| srvc.id)
            .await;

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
            assert_eq!(service.unwrap(), id);
        }
    }
}
