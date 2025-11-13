use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum_core::extract::FromRef;

type ApplyFn<S> = Box<dyn Send + Sync + Fn(Router, &S) -> RoadsterResult<Router>>;

/// A [`Middleware`] that can be applied without creating a separate `struct`. Useful to easily
/// apply a middleware that's based on a function, for example.
///
/// # Examples
/// ```rust
/// # use axum::response::Response;
/// # use axum::middleware::Next;
/// # use axum_core::extract::Request;
/// # use tracing::info;
/// # use roadster::service::http::middleware::any::AnyMiddleware;
/// #
/// pub(crate) async fn hello_world_middleware_fn(request: Request, next: Next) -> Response {
///     info!("Running `hello-world` middleware");
///
///     next.run(request).await
/// }
///
/// let middleware = AnyMiddleware::builder()
///     .name("hello-world")
///     .enabled(true)
///     .apply(|router, _state| {
///         Ok(router
///             .layer(axum::middleware::from_fn(hello_world_middleware_fn)))
///     })
///     .build();
/// ```
#[derive(bon::Builder)]
#[non_exhaustive]
pub struct AnyMiddleware<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    #[builder(into)]
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    #[builder(setters(vis = "", name = apply_internal))]
    apply: ApplyFn<S>,
}

impl<S, BS> AnyMiddlewareBuilder<S, BS>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    BS: any_middleware_builder::State,
{
    pub fn apply(
        self,
        apply_fn: impl 'static + Send + Sync + Fn(Router, &S) -> RoadsterResult<Router>,
    ) -> AnyMiddlewareBuilder<S, any_middleware_builder::SetApply<BS>>
    where
        BS::Apply: any_middleware_builder::IsUnset,
    {
        self.apply_internal(Box::new(apply_fn))
    }
}

impl<S> Middleware<S> for AnyMiddleware<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        // If the field on `AnyMiddleware` is set, use that
        if let Some(enabled) = self.enabled {
            return enabled;
        }

        let context = AppContext::from_ref(state);
        let custom_config = context
            .config()
            .service
            .http
            .custom
            .middleware
            .custom
            .get(&self.name);

        if let Some(custom_config) = custom_config {
            custom_config.common.enabled(state)
        } else {
            context
                .config()
                .service
                .http
                .custom
                .middleware
                .default_enable
        }
    }

    fn priority(&self, state: &S) -> i32 {
        // If the field on `AnyMiddleware` is set, use that
        if let Some(priority) = self.priority {
            return priority;
        }

        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .custom
            .get(&self.name)
            .map(|config| config.common.priority)
            .unwrap_or_default()
    }

    fn install(&self, state: &S, router: Router) -> Result<Router, Self::Error> {
        (self.apply)(router, state)
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::service::http::middleware::{CommonConfig, MiddlewareConfig};
    use crate::config::{AppConfig, CustomConfig};
    use crate::service::http::middleware::Middleware;
    use crate::service::http::middleware::any::AnyMiddleware;
    use crate::testing::snapshot::TestCase;
    use rstest::{fixture, rstest};

    const NAME: &str = "hello-world";

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn name() {
        let middleware = AnyMiddleware::builder()
            .name(NAME)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(middleware.name(), NAME);
    }

    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, None, Some(false), false)]
    #[case(false, Some(false), None, false)]
    #[case(false, None, Some(true), true)]
    #[case(false, Some(true), None, true)]
    #[case(true, None, Some(false), false)]
    #[case(true, Some(false), None, false)]
    #[case(true, None, None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enabled(
        _case: TestCase,
        #[case] default_enabled: bool,
        #[case] enabled_config: Option<bool>,
        #[case] enabled_field: Option<bool>,
        #[case] expected: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enabled;
        if let Some(enabled_config) = enabled_config {
            let middleware_config: MiddlewareConfig<CustomConfig> = MiddlewareConfig {
                common: CommonConfig {
                    enable: Some(enabled_config),
                    priority: 0,
                },
                custom: CustomConfig::default(),
            };
            config
                .service
                .http
                .custom
                .middleware
                .custom
                .insert(NAME.to_string(), middleware_config);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = AnyMiddleware::builder()
            .name(NAME)
            .maybe_enabled(enabled_field)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(middleware.enabled(&context), expected);
    }

    #[rstest]
    #[case(None, None, 0)]
    #[case(None, Some(10), 10)]
    #[case(Some(20), None, 20)]
    #[case(Some(20), Some(10), 10)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn priority(
        _case: TestCase,
        #[case] config_priority: Option<i32>,
        #[case] field_priority: Option<i32>,
        #[case] expected: i32,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        if let Some(config_priority) = config_priority {
            let middleware_config: MiddlewareConfig<CustomConfig> = MiddlewareConfig {
                common: CommonConfig {
                    enable: None,
                    priority: config_priority,
                },
                custom: CustomConfig::default(),
            };
            config
                .service
                .http
                .custom
                .middleware
                .custom
                .insert(NAME.to_string(), middleware_config);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = AnyMiddleware::builder()
            .name(NAME)
            .maybe_priority(field_priority)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(middleware.priority(&context), expected);
    }
}
