use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::initializer::Initializer;
use axum::Router;
use axum_core::extract::FromRef;
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct AnyInitializer<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    #[builder(default, setter(strip_option))]
    priority: Option<i32>,
    #[builder(default, setter(strip_option))]
    stage: Option<Stage>,
    #[builder(setter(transform = |a: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static| to_box_fn(a) ))]
    apply: Box<dyn Fn(Router, &S) -> RoadsterResult<Router> + Send>,
}

#[derive(Default)]
#[non_exhaustive]
pub enum Stage {
    AfterRouter,
    BeforeMiddleware,
    AfterMiddleware,
    #[default]
    BeforeServe,
}

fn to_box_fn<S>(
    p: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static,
) -> Box<dyn Fn(Router, &S) -> RoadsterResult<Router> + Send> {
    Box::new(p)
}

impl<S> Initializer<S> for AnyInitializer<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        let config = context
            .config()
            .service
            .http
            .custom
            .initializer
            .custom
            .get(&self.name);
        if let Some(config) = config {
            config.common.enabled(state)
        } else {
            context
                .config()
                .service
                .http
                .custom
                .initializer
                .default_enable
                || self.enabled.unwrap_or_default()
        }
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .initializer
            .custom
            .get(&self.name)
            .map(|config| config.common.priority)
            .unwrap_or_else(|| self.priority.unwrap_or_default())
    }

    fn after_router(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        if let Some(Stage::AfterRouter) = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn before_middleware(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        if let Some(Stage::BeforeMiddleware) = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn after_middleware(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        if let Some(Stage::AfterMiddleware) = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }

    fn before_serve(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        if let Some(Stage::BeforeServe) = self.stage {
            (self.apply)(router, _state)
        } else {
            Ok(router)
        }
    }
}
