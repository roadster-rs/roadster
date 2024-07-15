// todo: restructure out of the `extra` mod to something like
// - schema
//      - migration
//          - user
//          - role
//          - common? (for helpers like pk_uuid()) (where does seaorm put theirs?)

#[cfg(feature = "db-sql")]
pub mod migration;
#[cfg(feature = "db-sql")]
pub mod user;
