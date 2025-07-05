use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::Service;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use std::future::Future;
use std::marker::PhantomData;
use tokio_util::sync::CancellationToken;
use typed_builder::TypedBuilder;

/// A generic [Service] to allow creating a service from an async function.
///
/// # Examples
/// ```rust
/// # use async_trait::async_trait;
/// # use tokio_util::sync::CancellationToken;
/// # use roadster::app::context::AppContext;
/// # use roadster::error::RoadsterResult;
/// # use roadster::service::function::service::FunctionService;
/// # use roadster::service::registry::ServiceRegistry;
/// # use roadster::app::RoadsterApp;
///
/// async fn example_service(
///     _state: AppContext,
///     _cancel_token: CancellationToken,
/// ) -> RoadsterResult<()> {
///     // Service logic here
/// #    unimplemented!()
/// }
///
/// type App = RoadsterApp<AppContext>;
///
/// let service = FunctionService::builder()
///             .name("example".to_string())
///             .enabled(true)
///             .function(example_service)
///             .build();
///
/// let app: App = RoadsterApp::builder()
///     .add_service(service)
///     .build();
/// ```
#[derive(TypedBuilder)]
pub struct FunctionService<S, F, Fut>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    F: Send + Sync + Fn(S, CancellationToken) -> Fut,
    Fut: Send + Future<Output = RoadsterResult<()>>,
{
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    function: F,
    #[builder(default, setter(skip))]
    _state: PhantomData<S>,
}

#[async_trait]
impl<S, F, Fut> Service<S> for FunctionService<S, F, Fut>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    F: 'static + Send + Sync + Fn(S, CancellationToken) -> Fut,
    Fut: 'static + Send + Future<Output = RoadsterResult<()>>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        self.enabled
            .unwrap_or(AppContext::from_ref(state).config().service.default_enable)
    }

    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        (self.function)(state.clone(), cancel_token).await
    }
}
