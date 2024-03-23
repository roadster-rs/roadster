use itertools::Itertools;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref DEFAULT_QUEUE_NAMES: Vec<String> =
        ["default"].iter().map(|s| s.to_string()).collect();
}

pub fn queue_names(custom_queue_names: &Vec<String>) -> Vec<String> {
    DEFAULT_QUEUE_NAMES
        .iter()
        .chain(custom_queue_names)
        .unique()
        .map(|s| s.to_owned())
        .collect()
}
