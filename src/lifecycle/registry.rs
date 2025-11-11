use crate::app::context::AppContext;
use crate::app::{App, PreparedAppWithoutCli};
use crate::error::RoadsterResult;
use crate::lifecycle::AppLifecycleHandler;
use crate::lifecycle::default::default_lifecycle_handlers;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LifecycleHandlerRegistryError {
    /// The provided [`AppLifecycleHandler`] was already registered. Contains the
    /// [`AppLifecycleHandler::name`] of the provided service.
    #[error("The provided `AppLifecycleHandler` was already registered: `{0}`")]
    AlreadyRegistered(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Registry for the app's [`AppLifecycleHandler`]s.
///
/// # Examples
/// ```rust
/// # use std::convert::Infallible;
/// use async_trait::async_trait;
/// # use tokio_util::sync::CancellationToken;
/// # use roadster::app::context::AppContext;
/// # use roadster::error::RoadsterResult;
/// # use roadster::service::function::service::FunctionService;
/// # use roadster::service::registry::ServiceRegistry;
/// # use roadster::app::RoadsterApp;
/// # use roadster::lifecycle::AppLifecycleHandler;
/// # use roadster::lifecycle::registry::LifecycleHandlerRegistry;
/// #
/// struct ExampleLifecycleHandler;
///
/// type App = RoadsterApp<AppContext>;
///
/// impl AppLifecycleHandler<App, AppContext> for ExampleLifecycleHandler {
///     type Error = Infallible;
///
///     fn name(&self) -> String {
///         "example".to_string()
///     }
/// }
///
/// let app: App = RoadsterApp::builder()
///     .add_lifecycle_handler(ExampleLifecycleHandler)
///     // ...
///     .build();
/// ```
pub struct LifecycleHandlerRegistry<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    state: S,
    handlers: BTreeMap<String, Box<dyn AppLifecycleHandler<A, S, Error = crate::error::Error>>>,
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
    /// (e.g., [`AppLifecycleHandler::enabled`] returns `false`), the [`AppLifecycleHandler`]
    /// will not be registered.
    pub fn register<H>(&mut self, handler: H) -> RoadsterResult<()>
    where
        H: 'static + AppLifecycleHandler<A, S>,
    {
        self.register_boxed(Box::new(AppLifecycleHandlerWrapper::<A, S>::new(handler)))
    }

    pub(crate) fn register_boxed(
        &mut self,
        handler: Box<dyn AppLifecycleHandler<A, S, Error = crate::error::Error>>,
    ) -> RoadsterResult<()> {
        let name = handler.name();

        if !handler.enabled(&self.state) {
            info!(lifecycle_handler.name=%name, "Lifecycle handler is not enabled, skipping registration");
            return Ok(());
        }

        info!(lifecycle_handler.name=%name, "Registering lifecycle handler");

        if self.handlers.insert(name.clone(), handler).is_some() {
            return Err(LifecycleHandlerRegistryError::AlreadyRegistered(name).into());
        }

        Ok(())
    }

    /// Get the registered [`AppLifecycleHandler`]s, ordered by their
    /// [`AppLifecycleHandler::priority`].
    pub(crate) fn handlers(
        &self,
        state: &S,
    ) -> Vec<&dyn AppLifecycleHandler<A, S, Error = crate::error::Error>> {
        self.handlers
            .values()
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            .map(|handler| handler.deref())
            .collect_vec()
    }
}

type EnabledFn<S> = Box<dyn Send + Sync + for<'a> Fn(&'a S) -> bool>;
type PriorityFn<S> = Box<dyn Send + Sync + for<'a> Fn(&'a S) -> i32>;

type FnWithPreparedApp<A, S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a PreparedAppWithoutCli<A, S>,
        )
            -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

type OnShutdownFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S) -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

pub(crate) struct AppLifecycleHandlerWrapper<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    name: String,
    enabled_fn: EnabledFn<S>,
    priority_fn: PriorityFn<S>,
    before_health_checks_fn: FnWithPreparedApp<A, S>,
    before_services_fn: FnWithPreparedApp<A, S>,
    on_shutdown_fn: OnShutdownFn<S>,
    _phantom_data: PhantomData<A>,
}

impl<A, S> AppLifecycleHandlerWrapper<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    pub(crate) fn new<H>(handler: H) -> Self
    where
        H: 'static + AppLifecycleHandler<A, S>,
    {
        let name = handler.name();
        let handler = Arc::new(handler);
        let enabled_fn: EnabledFn<S> = {
            let handler = handler.clone();
            Box::new(move |state| handler.enabled(state))
        };
        let priority_fn: PriorityFn<S> = {
            let handler = handler.clone();
            Box::new(move |state| handler.priority(state))
        };
        let before_health_checks_fn: FnWithPreparedApp<A, S> = {
            let handler = handler.clone();
            Box::new(move |prepared| {
                let handler = handler.clone();
                Box::pin(async move {
                    handler
                        .before_health_checks(prepared)
                        .await
                        .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                    Ok(())
                })
            })
        };
        let before_services_fn: FnWithPreparedApp<A, S> = {
            let handler = handler.clone();
            Box::new(move |prepared| {
                let handler = handler.clone();
                Box::pin(async move {
                    handler
                        .before_services(prepared)
                        .await
                        .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                    Ok(())
                })
            })
        };
        let on_shutdown_fn: OnShutdownFn<S> = {
            let handler = handler.clone();
            Box::new(move |state| {
                let handler = handler.clone();
                Box::pin(async move {
                    handler
                        .on_shutdown(state)
                        .await
                        .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                    Ok(())
                })
            })
        };
        Self {
            name,
            enabled_fn,
            priority_fn,
            before_health_checks_fn,
            before_services_fn,
            on_shutdown_fn,
            _phantom_data: Default::default(),
        }
    }
}

#[async_trait]
impl<A, S> AppLifecycleHandler<A, S> for AppLifecycleHandlerWrapper<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        (self.enabled_fn)(state)
    }

    fn priority(&self, state: &S) -> i32 {
        (self.priority_fn)(state)
    }

    async fn before_health_checks(
        &self,
        prepared_app: &PreparedAppWithoutCli<A, S>,
    ) -> Result<(), Self::Error> {
        (self.before_health_checks_fn)(prepared_app).await
    }

    async fn before_services(
        &self,
        prepared_app: &PreparedAppWithoutCli<A, S>,
    ) -> Result<(), Self::Error> {
        (self.before_services_fn)(prepared_app).await
    }

    async fn on_shutdown(&self, state: &S) -> Result<(), Self::Error> {
        (self.on_shutdown_fn)(state).await
    }
}
