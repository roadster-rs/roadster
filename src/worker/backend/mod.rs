use crate::config::service::worker::QueueConfig;
use itertools::Itertools;
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
