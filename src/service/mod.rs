use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use std::any::Any;
use tokio_util::sync::CancellationToken;

pub mod function;
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "http")]
pub mod http;
pub mod registry;
pub(crate) mod runner;
pub mod worker;

/// Trait to represent a service (e.g., a persistent task) to run in the app. Example services
/// include, but are not limited to: an [http API][crate::service::http::service::HttpService],
/// a sidekiq processor, or a gRPC API.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AppService<A, S>: Send + Sync + AppServiceAsAny<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    /// The name of the service.
    fn name(&self) -> String;

    /// Whether the service is enabled. If the service is not enabled, it will not be run.
    fn enabled(&self, state: &S) -> bool;

    /// Perform any initialization work or other checks that should be done before the service runs.
    ///
    /// For example, checking that the service is healthy, removing stale items from the
    /// service's queue, etc.
    async fn before_run(&self, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }

    /// Run the service in a new tokio task.
    ///
    /// * cancel_token - A tokio [CancellationToken] to use as a signal to gracefully shut down
    /// the service.
    async fn run(self: Box<Self>, state: &S, cancel_token: CancellationToken)
    -> RoadsterResult<()>;
}

/// Trait used to build an [AppService]. It's not a requirement that services implement this
/// trait; it is provided as a convenience. A [builder][AppServiceBuilder] can be provided to
/// the [ServiceRegistry][crate::service::registry::ServiceRegistry] instead of an [AppService],
/// in which case the [ServiceRegistry][crate::service::registry::ServiceRegistry] will only
/// build and register the service if [AppService::enabled] is `true`.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AppServiceBuilder<A, S, Service>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    Service: AppService<A, S>,
{
    fn name(&self) -> String;

    fn enabled(&self, state: &S) -> bool;

    async fn build(self, state: &S) -> RoadsterResult<Service>;
}

/// Allows getting an `&dyn Any` reference to the [`AppService`]. This is to enable getting
/// a concrete reference to a specific [`AppService`] implementation from the
/// [`registry::ServiceRegistry`] (which works via a downcast) in order to call methods that are
/// specific to a certain implementation. See [`registry::ServiceRegistry::get`] for more details
/// and examples.
/*
A note for future maintainers: This `AppService`-specific trait is required to get a `&dyn Any`
for the service because the following don't work:

1. General `AsAny` trait with a global impl -- doesn't work because this also implements `AsAny`
   for `Box`, which prevents us from getting `&dyn Any` for the actual `AppService` that we want.
```rust
trait AsAny {
    fn as_any(&self) -> &dyn Any;
}
impl<T> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
```

2. General `AsAny` trait only implemented for specific traits, e.g. `AppService` -- doesn't work
   for `AppService` because Rust considers the `A` and `S` type parameters as unconstrained.
```rust
trait AsAny {
    fn as_any(&self) -> &dyn Any;
}
impl<T, A, S> AsAny for T
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    // Even though `A` and `S` appear here, Rust considers them unconstrained.
    T: AppService<A, S>
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
```
*/
pub trait AppServiceAsAny<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn as_any(&self) -> &dyn Any;
}

/// Provide an auto-impl of [`AppServiceAsAny`] for any type that implements [`AppService`].
impl<T, A, S> AppServiceAsAny<A, S> for T
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    T: AppService<A, S> + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}
