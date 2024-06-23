use crate::app::context::AppContext;
use crate::app::App;
#[cfg(feature = "db-sql")]
use crate::health_check::database::DatabaseHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq::SidekiqHealthCheck;
use crate::health_check::HealthCheck;
use std::collections::BTreeMap;

pub fn default_health_checks<A: App + 'static>(
    context: &AppContext<A::State>,
) -> BTreeMap<String, Box<dyn HealthCheck<A>>> {
    let health_check: Vec<Box<dyn HealthCheck<A>>> = vec![
        #[cfg(feature = "db-sql")]
        Box::new(DatabaseHealthCheck),
        #[cfg(feature = "sidekiq")]
        Box::new(SidekiqHealthCheck),
    ];
    health_check
        .into_iter()
        .filter(|health_check| health_check.enabled(context))
        .map(|health_check| (health_check.name(), health_check))
        .collect()
}
