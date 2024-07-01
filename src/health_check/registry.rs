use crate::error::RoadsterResult;
use crate::health_check::HealthCheck;
use anyhow::anyhow;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// Registry for [HealthCheck]s that will be run in the app.
///
/// Health checks are used in multiple parts of the app, for example:
/// 1. As pre-boot checks to ensure the app's resource dependencies are healthy.
/// 2. As a "core" API that can be used from multiple components, e.g. the `_health` HTTP endpoint
///    and the health CLI command.
///
/// # Internal mutability
/// In order to make this registry available to multiple parts of the app, this is included
/// as part of the [AppContext][crate::app::context::AppContext]. This is not strictly necessary
/// for the Axum handlers (the registry could be provided via an [Extension][axum::Extension]),
/// but it is (currently) required for other things, such as the CLI handlers.
///
/// In order to include the registry as part of the context, but also allow checks to be added
/// to the registry after the context is created, the registry implements the
/// [interior mutability](https://doc.rust-lang.org/reference/interior-mutability.html) pattern
/// using a [RwLock]. As such, ___it is not recommended to register additional health checks
/// outside of the app initialization process___ -- doing so may result in a panic.
///
/// Because of the internal mutability, methods that modify the internal state can accept `&self`
/// instead of `&mut self`.
pub struct HealthCheckRegistry {
    health_checks: Arc<RwLock<BTreeMap<String, Arc<dyn HealthCheck>>>>,
}

impl Default for HealthCheckRegistry {
    fn default() -> Self {
        HealthCheckRegistry::new()
    }
}

impl HealthCheckRegistry {
    pub fn new() -> Self {
        Self {
            health_checks: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub fn register<H>(&self, health_check: H) -> RoadsterResult<()>
    where
        H: HealthCheck + 'static,
    {
        self.register_arc(Arc::new(health_check))
    }

    pub(crate) fn register_arc(&self, health_check: Arc<dyn HealthCheck>) -> RoadsterResult<()> {
        let name = health_check.name();

        if !health_check.enabled() {
            info!(name=%name, "Health check is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering health check");

        let mut health_checks = self.health_checks.write().map_err(|err| {
            anyhow!("Unable to acquire write lock on health check registry: {err}")
        })?;
        if health_checks.insert(name.clone(), health_check).is_some() {
            return Err(anyhow!("Health check `{}` was already registered", name).into());
        }
        Ok(())
    }

    pub fn checks(&self) -> RoadsterResult<Vec<Arc<dyn HealthCheck>>> {
        let health_checks = self
            .health_checks
            .read()
            .map_err(|err| anyhow!("Unable to acquire read lock on heath check registry: {err}"))?;
        Ok(health_checks.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_check::MockHealthCheck;
    use rstest::rstest;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 0)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_check(#[case] service_enabled: bool, #[case] expected_count: usize) {
        // Arrange
        let mut check: MockHealthCheck = MockHealthCheck::default();
        check.expect_enabled().return_const(service_enabled);
        check.expect_name().return_const("test".to_string());

        // Act
        let subject: HealthCheckRegistry = HealthCheckRegistry::new();
        subject.register(check).unwrap();

        // Assert
        assert_eq!(subject.checks().unwrap().len(), expected_count);
        assert_eq!(
            subject
                .checks()
                .unwrap()
                .iter()
                .any(|check| check.name() == "test"),
            service_enabled
        );
    }
}
