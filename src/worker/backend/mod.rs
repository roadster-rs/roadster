#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

#[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
fn shared_queues<'a>(
    config_queues: &'a Option<std::collections::BTreeSet<String>>,
    all_queues: &'a std::collections::BTreeSet<String>,
    dedicated_queues: &'a std::collections::BTreeMap<
        String,
        crate::config::service::worker::QueueConfig,
    >,
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(None, Default::default(), Default::default())]
    #[case(Some(BTreeSet::from(["foo".to_owned()])), Default::default(), Default::default())]
    #[case(None, BTreeSet::from(["foo".to_owned()]), Default::default())]
    #[case(Some(BTreeSet::from(["foo".to_owned()])), BTreeSet::from(["bar".to_owned()]), Default::default())]
    #[case(Some(BTreeSet::from(["foo".to_owned()])), Default::default(), [("foo".to_string(), Default::default())].into_iter().collect())]
    #[case(Some(BTreeSet::from(["foo".to_owned(), "bar".to_owned()])), Default::default(), [("foo".to_string(), Default::default())].into_iter().collect())]
    #[cfg(any(feature = "worker-sidekiq", feature = "worker-pg"))]
    #[cfg_attr(coverage_nightly, coverage(off))]
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
