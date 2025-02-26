use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum_core::extract::FromRef;
use typed_builder::TypedBuilder;

type ApplyFn<S> = Box<dyn Fn(Router, &S) -> RoadsterResult<Router> + Send>;

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
#[derive(TypedBuilder)]
#[non_exhaustive]
pub struct AnyMiddleware<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option(fallback = enabled_opt)))]
    enabled: Option<bool>,
    #[builder(default, setter(strip_option(fallback = priority_opt)))]
    priority: Option<i32>,
    #[builder(setter(transform = |a: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static| to_box_fn(a) ))]
    apply: ApplyFn<S>,
}

fn to_box_fn<S>(p: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static) -> ApplyFn<S> {
    Box::new(p)
}

impl<S> Middleware<S> for AnyMiddleware<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
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

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
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
            .enabled_opt(enabled_field)
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
            .priority_opt(field_priority)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(middleware.priority(&context), expected);
    }
}
