use crate::config::service::worker::QueueConfig;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

fn shared_queues<'a>(
    config_queues: &'a Option<BTreeSet<String>>,
    all_queues: &'a BTreeSet<String>,
    dedicated_queues: &'a BTreeMap<String, QueueConfig>,
) -> impl Iterator<Item = &'a String> {
    config_queues
        .as_ref()
        .unwrap_or(all_queues)
        .iter()
        .filter(|queue| !dedicated_queues.contains_key(*queue))
}

#[cfg(test)]
mod tests {
    use crate::config::service::worker::QueueConfig;
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};
    use std::collections::{BTreeMap, BTreeSet};

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    // Todo: more cases
    #[rstest]
    #[case(None, Default::default(), Default::default())]
    #[case(Some(BTreeSet::from(["foo".to_owned()])), Default::default(), Default::default())]
    fn shared_queues(
        _case: TestCase,
        #[case] config_queues: Option<BTreeSet<String>>,
        #[case] all_queues: BTreeSet<String>,
        #[case] dedicated_queues: BTreeMap<String, QueueConfig>,
    ) {
        let shared_queues =
            super::shared_queues(&config_queues, &all_queues, &dedicated_queues).collect_vec();
        assert_debug_snapshot!(shared_queues);
    }
}
