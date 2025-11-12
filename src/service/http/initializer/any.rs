use crate::app::context::AppContext;
use crate::service::http::initializer::Initializer;
use axum::Router;
use axum_core::extract::FromRef;

type ApplyFn<S, E> = Box<dyn Send + Sync + Fn(Router, &S) -> Result<Router, E> + Send>;

/// An [`Initializer`] that can be applied without creating a separate `struct`.
///
/// # Examples
/// ```rust
/// # use axum::response::Response;
/// # use axum::middleware::Next;
/// # use axum_core::extract::Request;
/// # use tracing::info;
/// # use roadster::service::http::initializer::any::AnyInitializer;
/// # use roadster::app::context::AppContext;
/// # use std::convert::Infallible;
/// #
/// AnyInitializer::<AppContext, Infallible>::builder()
///     .name("hello-world")
///     .stage(roadster::service::http::initializer::any::Stage::BeforeServe)
///     .apply(|router, _state| {
///         info!("Running `hello-world` initializer");
///         Ok(router)
///     })
///     .build();
/// ```
#[derive(bon::Builder)]
pub struct AnyInitializer<S, E>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    E: Send + Sync + std::error::Error,
{
    #[builder(into)]
    name: String,
    enabled: Option<bool>,
    priority: Option<i32>,
    #[builder(default)]
    stage: Stage,
    #[builder(setters(vis = "", name = apply_internal))]
    apply: ApplyFn<S, E>,
}

impl<S, E, BS> AnyInitializerBuilder<S, E, BS>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    E: Send + Sync + std::error::Error,
    BS: any_initializer_builder::State,
{
    pub fn apply(
        self,
        apply_fn: impl 'static + Send + Sync + Fn(Router, &S) -> Result<Router, E>,
    ) -> AnyInitializerBuilder<S, E, any_initializer_builder::SetApply<BS>>
    where
        BS::Apply: any_initializer_builder::IsUnset,
    {
        self.apply_internal(Box::new(apply_fn))
    }
}

#[derive(Default)]
#[non_exhaustive]
pub enum Stage {
    AfterRouter,
    BeforeMiddleware,
    #[default]
    AfterMiddleware,
    BeforeServe,
}

impl<S, E> Initializer<S> for AnyInitializer<S, E>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    E: Send + Sync + std::error::Error,
{
    type Error = E;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        // If the field on `AnyInitializer` is set, use that
        if let Some(enabled) = self.enabled {
            return enabled;
        }

        let context = AppContext::from_ref(state);
        let custom_config = context
            .config()
            .service
            .http
            .custom
            .initializer
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
                .initializer
                .default_enable
        }
    }

    fn priority(&self, state: &S) -> i32 {
        // If the field on `AnyInitializer` is set, use that
        if let Some(priority) = self.priority {
            return priority;
        }

        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .initializer
            .custom
            .get(&self.name)
            .map(|config| config.common.priority)
            .unwrap_or_default()
    }

    fn after_router(&self, router: Router, _state: &S) -> Result<Router, Self::Error> {
        if let Stage::AfterRouter = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn before_middleware(&self, router: Router, _state: &S) -> Result<Router, Self::Error> {
        if let Stage::BeforeMiddleware = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn after_middleware(&self, router: Router, _state: &S) -> Result<Router, Self::Error> {
        if let Stage::AfterMiddleware = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn before_serve(&self, router: Router, _state: &S) -> Result<Router, Self::Error> {
        if let Stage::BeforeServe = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::service::http::initializer::{CommonConfig, InitializerConfig};
    use crate::config::{AppConfig, CustomConfig};
    use crate::service::http::initializer::Initializer;
    use crate::service::http::initializer::any::AnyInitializer;
    use crate::testing::snapshot::TestCase;
    use rstest::{fixture, rstest};
    use std::convert::Infallible;

    const NAME: &str = "hello-world";

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn name() {
        let initializer = AnyInitializer::<AppContext, Infallible>::builder()
            .name(NAME)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(initializer.name(), NAME);
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
        config.service.http.custom.initializer.default_enable = default_enabled;
        if let Some(enabled_config) = enabled_config {
            let initializer_config: InitializerConfig<CustomConfig> = InitializerConfig {
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
                .initializer
                .custom
                .insert(NAME.to_string(), initializer_config);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        let initializer = AnyInitializer::<AppContext, Infallible>::builder()
            .name(NAME)
            .maybe_enabled(enabled_field)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(initializer.enabled(&context), expected);
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
            let initializer_config: InitializerConfig<CustomConfig> = InitializerConfig {
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
                .initializer
                .custom
                .insert(NAME.to_string(), initializer_config);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        let initializer = AnyInitializer::<AppContext, Infallible>::builder()
            .name(NAME)
            .maybe_priority(field_priority)
            .apply(|router, _state| Ok(router))
            .build();

        assert_eq!(initializer.priority(&context), expected);
    }
}
