use std::any::type_name;

/// Get the name of the type with its module prefix stripped.
pub fn simple_type_name<T>() -> String
where
{
    type_name::<T>()
        .split("::")
        .last()
        .unwrap_or(type_name::<T>())
        .to_owned()
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::service::worker::Worker;
    use crate::util;
    use insta::_macro_support::assert_snapshot;
    use insta::assert_debug_snapshot;
    use serde_derive::{Deserialize, Serialize};
    use std::time::Duration;

    struct Foo;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn simple_type_name() {
        let simple_type_name = util::simple_type_name::<Foo>();
        assert_debug_snapshot!(simple_type_name);
    }
}
