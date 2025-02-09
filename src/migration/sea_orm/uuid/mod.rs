use sea_orm::{DbBackend, Statement};

pub mod m20240726_062011_uuid_ossp_extension;

/// Create a [Statement] to create/enable the `uuid-ossp` Postgres extension
pub fn create_uuid_ossp_extension() -> Statement {
    Statement::from_string(
        DbBackend::Postgres,
        r#"CREATE EXTENSION IF NOT EXISTS "uuid-ossp";"#,
    )
}

/// Create a [Statement] to drop/disable the `uuid-ossp` Postgres extension.
pub fn drop_uuid_ossp_extension() -> Statement {
    Statement::from_string(
        DbBackend::Postgres,
        r#"DROP EXTENSION IF EXISTS "uuid-ossp";"#,
    )
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_uuid_ossp_extension() {
        let statement = super::create_uuid_ossp_extension();

        assert_debug_snapshot!(statement);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_uuid_ossp_extension() {
        let statement = super::drop_uuid_ossp_extension();

        assert_debug_snapshot!(statement);
    }
}
