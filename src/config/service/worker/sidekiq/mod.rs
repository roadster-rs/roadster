use crate::config::service::worker::{
    BalanceStrategy, Periodic, QueueConfig, StaleCleanUpBehavior,
};
use serde_derive::{Deserialize, Serialize};
use url::Url;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SidekiqServiceConfig {
    #[validate(nested)]
    pub redis: Redis,

    #[serde(default)]
    #[validate(nested)]
    pub periodic: Periodic,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Redis {
    pub uri: Url,

    /// The configuration for the Redis connection pool used for enqueuing Sidekiq jobs in Redis.
    #[serde(default)]
    #[validate(nested)]
    pub enqueue_pool: ConnectionPool,

    /// The configuration for the Redis connection pool used by [sidekiq::Processor] to fetch
    /// Sidekiq jobs from Redis.
    #[serde(default)]
    #[validate(nested)]
    pub fetch_pool: ConnectionPool,

    /// Options for creating a Test Container instance for the DB. If enabled, the `Redis#uri`
    /// field will be overridden to be the URI for the Test Container instance that's created when
    /// building the app's [`crate::app::context::AppContext`].
    #[cfg(feature = "test-containers")]
    #[serde(default)]
    pub test_container: Option<crate::config::TestContainer>,
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ConnectionPool {
    pub min_idle: Option<u32>,
    pub max_connections: Option<u32>,
}

#[derive(Default, Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Periodic {
    #[serde(default)]
    pub stale_cleanup: StaleCleanUpBehavior,
}

impl From<BalanceStrategy> for sidekiq::BalanceStrategy {
    fn from(value: BalanceStrategy) -> Self {
        match value {
            BalanceStrategy::RoundRobin => sidekiq::BalanceStrategy::RoundRobin,
            BalanceStrategy::None => sidekiq::BalanceStrategy::None,
        }
    }
}

impl From<&QueueConfig> for sidekiq::QueueConfig {
    fn from(value: &QueueConfig) -> Self {
        value
            .num_workers
            .iter()
            .fold(Default::default(), |config, num_workers| {
                config.num_workers(*num_workers as usize)
            })
    }
}

impl From<QueueConfig> for sidekiq::QueueConfig {
    fn from(value: QueueConfig) -> Self {
        sidekiq::QueueConfig::from(&value)
    }
}

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use ::sidekiq::BalanceStrategy as SidekiqBalanceStrategy;
    use ::sidekiq::QueueConfig as SidekiqQueueConfig;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        [redis]
        uri = "redis://localhost:6379"
        "#
    )]
    #[case(
        r#"
        [redis]
        uri = "redis://localhost:6379"
        [redis.enqueue-pool]
        min-idle = 1
        [redis.fetch-pool]
        min-idle = 2
        "#
    )]
    #[case(
        r#"
        [redis]
        uri = "redis://localhost:6379"
        [redis.enqueue-pool]
        max-connections = 1
        [redis.fetch-pool]
        max-connections = 2
        "#
    )]
    #[case(
        r#"
        [redis]
        uri = "redis://localhost:6379"
        [periodic]
        stale-cleanup = "auto-clean-stale"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sidekiq(_case: TestCase, #[case] config: &str) {
        let sidekiq: SidekiqServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sidekiq);
    }

    #[rstest]
    #[case(BalanceStrategy::RoundRobin)]
    #[case(BalanceStrategy::None)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn balance_strat_to_sidekiq_balance_strat(#[case] strategy: BalanceStrategy) {
        let sidekiq_strategy: SidekiqBalanceStrategy = strategy.clone().into();
        match sidekiq_strategy {
            SidekiqBalanceStrategy::RoundRobin => {
                assert!(matches!(strategy, BalanceStrategy::RoundRobin))
            }
            SidekiqBalanceStrategy::None => {
                assert!(matches!(strategy, BalanceStrategy::None))
            }
            _ => unimplemented!(),
        }
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn queue_config_to_sidekiq_queue_config() {
        let num_workers = 10;
        let config = QueueConfig {
            num_workers: Some(num_workers),
        };
        let sidekiq_config: SidekiqQueueConfig = config.into();
        assert_eq!(sidekiq_config.num_workers, num_workers as usize);
    }
}
