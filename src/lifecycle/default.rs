use crate::app::App;
use crate::app::context::AppContext;
use crate::lifecycle::AppLifecycleHandler;
use axum_core::extract::FromRef;
use std::collections::BTreeMap;

pub fn default_lifecycle_handlers<A, S>(
    state: &S,
) -> BTreeMap<String, Box<dyn AppLifecycleHandler<A, S, Error = crate::error::Error>>>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: 'static + App<S>,
{
    let lifecycle_handlers: Vec<Box<dyn AppLifecycleHandler<A, S, Error = crate::error::Error>>> = vec![
        #[cfg(feature = "db-sql")]
        Box::new(crate::lifecycle::db::migration::DbMigrationLifecycleHandler),
        #[cfg(feature = "db-sea-orm")]
        Box::new(
            crate::lifecycle::db::sea_orm::graceful_shutdown::DbSeaOrmGracefulShutdownLifecycleHandler,
        ),
    ];

    lifecycle_handlers
        .into_iter()
        .filter(|handler| handler.enabled(state))
        .map(|handler| (handler.name(), handler))
        .collect()
}
