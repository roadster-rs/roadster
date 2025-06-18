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
    use insta::assert_debug_snapshot;

    struct Foo;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn simple_type_name() {
        let simple_type_name = super::simple_type_name::<Foo>();
        assert_debug_snapshot!(simple_type_name);
    }
}
