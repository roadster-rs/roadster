// We need to use the disallowed `roadster::app_context::AppContext` type in this module in order
// to implement the required traits used to convert it to/from `AppState`.
#![allow(clippy::disallowed_types)]

pub type AppState = ();
