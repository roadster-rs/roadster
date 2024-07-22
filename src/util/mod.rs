pub mod regex;
pub mod serde;

#[deprecated(
    since = "0.5.6",
    note = "The `serde_util` module was renamed to `serde`"
)]
pub use serde as serde_util;
