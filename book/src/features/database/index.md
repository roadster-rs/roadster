# Database

When the `db-sql` feature is enabled, Roadster provides support for various SQL databases
via [SeaORM](https://docs.rs/sea-orm/1.1.4/sea_orm/), an ORM built on top of [sqlx](https://docs.rs/sqlx/latest/sqlx/).
See the SeaORM docs for more details.

## Migration

If you want to run your SeaORM migrations with Roadster, you need to provide
your [MigratorTrait](https://docs.rs/sea-orm-migration/1.1.4/sea_orm_migration/migrator/trait.MigratorTrait.html) type
to Roadster. This is done by setting the `M` associated type on
your [App](https://docs.rs/roadster/latest/roadster/app/trait.App.html) impl

```rust,ignore
{{#include ../../../examples/database/src/app.rs:6:}}
```

or the type parameter on
your [RoadsterApp](https://docs.rs/roadster/latest/roadster/app/struct.RoadsterApp.html) instance.

```rust,ignore
{{#include ../../../examples/database/src/roadster_app.rs:6:}}
```

### Run automatically

Roadster can automatically run your SeaORM migrations when your app is starting. This is enabled by default but can be
disabled via the
[database.auto-migrate](https://docs.rs/roadster/latest/roadster/config/database/struct.Database.html#structfield.auto_migrate)
config field.

```toml
[database]
auto-migrate = false
```

### Run via CLI

You can also manually run migrations via the CLI (when the `cli` feature is enabled).

```shell
cargo run -- roadster migrate up
```

## Docs.rs links

- <https://docs.rs/roadster/latest/roadster/migration/index.html>
- <https://docs.rs/roadster/latest/roadster/migration/check/index.html>
- <https://docs.rs/roadster/latest/roadster/migration/schema/index.html>
- <https://docs.rs/roadster/latest/roadster/migration/uuid/index.html>
