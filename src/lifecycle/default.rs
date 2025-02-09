use crate::app::context::AppContext;
use crate::app::App;
use crate::lifecycle::AppLifecycleHandler;
use axum_core::extract::FromRef;
use std::collections::BTreeMap;

pub fn default_lifecycle_handlers<A, S>(
    state: &S,
) -> BTreeMap<String, Box<dyn AppLifecycleHandler<A, S>>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    let lifecycle_handlers: Vec<Box<dyn AppLifecycleHandler<A, S>>> = vec![
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
