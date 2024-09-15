use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health_check::default::default_health_checks;
use crate::health_check::HealthCheck;
use anyhow::anyhow;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::info;

/// Registry for [HealthCheck]s that will be run in the app.
///
/// Health checks are used in multiple parts of the app, for example:
/// 1. As pre-boot checks to ensure the app's resource dependencies are healthy.
/// 2. As a "core" API that can be used from multiple components, e.g. the `_health` HTTP endpoint
///    and the health CLI command.
pub struct HealthCheckRegistry {
    health_checks: BTreeMap<String, Arc<dyn HealthCheck>>,
}

impl HealthCheckRegistry {
    pub(crate) fn new(context: &AppContext) -> Self {
        Self {
            health_checks: default_health_checks(context),
        }
    }

    pub fn register<H>(&mut self, health_check: H) -> RoadsterResult<()>
    where
        H: HealthCheck + 'static,
    {
        self.register_arc(Arc::new(health_check))
    }

    pub fn register_arc(&mut self, health_check: Arc<dyn HealthCheck>) -> RoadsterResult<()> {
        let name = health_check.name();

        if !health_check.enabled() {
            info!(name=%name, "Health check is not enabled, skipping registration");
            return Ok(());
        }

        info!(name=%name, "Registering health check");

        if self
            .health_checks
            .insert(name.clone(), health_check)
            .is_some()
        {
            return Err(anyhow!("Health check `{}` was already registered", name).into());
        }
        Ok(())
    }

    pub fn checks(&self) -> Vec<Arc<dyn HealthCheck>> {
        self.health_checks.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_config::AppConfig;
    use crate::health_check::MockHealthCheck;
    use rstest::rstest;

    #[rstest]
    #[case(true, 1)]
    #[case(false, 0)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn register_check(#[case] check_enabled: bool, #[case] expected_count: usize) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.health_check.default_enable = false;
        let context = AppContext::test(Some(config), None, None).unwrap();

        let mut check: MockHealthCheck = MockHealthCheck::default();
        check.expect_enabled().return_const(check_enabled);
        check.expect_name().return_const("test".to_string());

        // Act
        let mut subject: HealthCheckRegistry = HealthCheckRegistry::new(&context);
        subject.register(check).unwrap();

        // Assert
        assert_eq!(subject.checks().len(), expected_count);
        assert_eq!(
            subject.checks().iter().any(|check| check.name() == "test"),
            check_enabled
        );
    }
}
