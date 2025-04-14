# Testing

<!-- Todo: add docs.rs links -->

Roadster provides various utilities to make it easier to test your app, including [`insta`](https://docs.rs/insta)
snapshot utilities, temporary DB creation, and the [
`run_test*`](https://docs.rs/roadster/0.7.0-beta/roadster/app/fn.run_test.html) methods that allow running tests against
a fully initialized app.

## Snapshot utilities

[`insta`](https://docs.rs/insta) is a popular crate that enables writing snapshot tests. `insta` allows configuring some
settings for how snapshots are generated. Roadster provides some default settings via the `TestCase` struct, which
in turn can be customized via the `TestCaseConfig` struct. `TestCase` automatically applies the configured `insta`
settings to the current `insta` test context when it's created.

### Redacting sensitive or dynamic fields

Snapshot testing relies on the content remaining the same between test runs. This means that dynamic data, such as
database IDs, need to be substituted with constant placeholder in order for the snapshot tests to pass.

In addition, because snapshots are checked in to source-control, it's also important that any sensitive data is redacted
from the snapshot in order to avoid leaking secrets.

Insta allows defining filters to redact data from snapshots based on regexes, and Roadster provides some default filters
via the `TestCase` struct, including redacting bearer tokens and postgres connection details and normalizing UUIDs and
timestamps.

#### Examples

Creating a snapshot with a db url

```rust,ignore
{{#include ../../examples/testing/src/snapshots/redact_db_url.rs:4:}}
```

Results in the following snapshot file:

```
{{#include ../../examples/testing/src/snapshots/snapshots/testing_example__snapshots__redact_db_url__redact_sensitive_db_url@redact_sensitive_db_url.snap}}
```

Creating a snapshot with a uuid

```rust,ignore
{{#include ../../examples/testing/src/snapshots/redact_uuid.rs:5:}}
```

Results in the following snapshot file:

```
{{#include ../../examples/testing/src/snapshots/snapshots/testing_example__snapshots__redact_uuid__normalize_dynamic_uuid@normalize_dynamic_uuid.snap}}
```

### `insta` + `rstest`

The [`rstest`](https://docs.rs/rstest) crate allows writing tests using a concept known
as [test fixtures](https://en.wikipedia.org/wiki/Test_fixture#Software). Normally, `insta`'s default logic for
generating snapshot names doesn't work well with `rstest` -- `insta` uses the test name as the snapshot name, and
`rstest` causes the same test to run multiple times. This means each `rstest`-based test needs to have a different
snapshot name in order to avoid each `insta` invocation overwriting a previous snapshot for the test.

Roadster's `TestCase` struct allows customizing `insta`'s logic for generating snapshot names in a way that works well
with `rstest`. Roadster's logic appends either the `rstest` case number or name/description as a suffix to the snapshot
name. This allows `insta` create unique snapshot files for each `rstest` case.

#### Examples

Using a `TestCase` when using `insta` together with `rstest` to write parameterized snapshot tests

```rust,ignore
{{#include ../../examples/testing/src/snapshots/rstest.rs:5:}}
```

Generates the following snapshot files:

```text
<crate_name>__snapshots__rstest__normalize_dynamic_uuid@case_01.snap
<crate_name>__snapshots__rstest__normalize_dynamic_uuid@case_02.snap
<crate_name>__snapshots__rstest__normalize_dynamic_uuid@case_name.snap
```

## Testing with an initialized app

A majority of an app's test coverage may come from small, targeted unit tests. These are generally faster to run and
easier to write because they test, for example, only a specific function's behavior and use fake/mock data for
everything else. However, an app will usually want some level of end-to-end (E2E) testing, where entire API endpoints
are tested via their request and response. An app may also want to write tests that interact with an actual DB, such
as testing the ORM's model for a table in the DB.

For these cases, Roadster provides the [`run_test`](https://docs.rs/roadster/0.7.0-beta/roadster/app/fn.run_test.html)
and [`run_test_with_result`](https://docs.rs/roadster/0.7.0-beta/roadster/app/fn.run_test_with_result.html) methods
to run a test with a fully initialized app. Both methods will initialize the app before running the provided test
closure, and tear down the app when the test closure completes. Note, however, that if the test closure panics, the
app may not be torn down. If it's vital that the app is town down on test failure, either set the `testing.catch-panic`
config to `true`, or use `run_test_with_result` and take care not to panic inside the test closure.

```rust,ignore
{{#include ../../../examples/app-builder/tests/ping.rs:8:}}
```

## Test isolation

In order to maintain an efficient test suite, it's important for tests to be stable and parallelizable. For tests that
work with the app's resources, such as a database, this requires special care to ensure tests do not conflict with each
other. For example, if two tests try to create a user with the same email in parallel, one of the tests may fail if the
database has a unique email constraint.

### Randomized data

Probably the easiest way to ensure database-related tests do not conflict is to use randomized data for all fields using
a crate such as [`fake`](https://docs.rs/fake). This crate allows creating plausible but fake/randomized data for
various types of data, such as emails, usernames, and passwords. Compared to the other approaches mentioned below, this
approach has the benefit of being the most efficient as no additional resources need to be initialized. However, this
approach requires diligence to ensure hard-coded/conflicting data is not used in tests. If a more fool-proof approach is
desired, the below approaches may be preferred. See the [`fake`](https://docs.rs/fake) docs for examples.

### Temporary DB

Creating a temporary DB for each test virtually guarantees that DB-related tests will never conflict with each other.
The downside is there is a small performance hit for each test due to the DB operations needed to initialize the
temporary DB. However, this may be worth the performance impact for the benefit of having stable, non-conflicting tests.
The temporary DB connection is made available via the normal DB connection methods on the `AppContext`.

When using the `run_test*` methods, Roadster allows creating a temporary DB for testing. If enabled, Roadster will
create a new DB for each test using the original DB connection details. If the `database.temporary-test-db-clean-up`
config is set to `true`, the temporary DB will be deleted when the test completes. Note, however, that if the closure
passed to the `run_test*` method(s) panics, the DB will not be deleted.

Note: This feature is only supported on Postgres and Mysql at the moment.

#### Examples

Example config to enable creating a temporary db in tests that use the `run_test*` methods

```toml
{{ #include ../../examples/testing/config/test/temp_db.toml}}
```

### Test Containers

In addition to the above temporary DB approach, temporary DBs (or any other external docker-based resource) can be
created and automatically torn down for each test using [Test Containers](https://testcontainers.com/). Roadster
provides built-in support for initializing a DB, Redis, or SMTP server instance via test containers. The test container
connection is made available via the normal DB connection methods on the `AppContext`.

Note that compared to the temporary DB solution discussed above, test containers have an additional performance hit due
to the operations needed to initialize a new docker container for each test container instance. This means that this is
the slowest option for ensuring tests are isolated. However, this solution supports other resources that your tests
may need to interact with besides just databases (e.g. Redis and SMPT servers).

#### Examples

Example config to enable [Test Containers](https://testcontainers.com/) in tests that use the `run_test*` methods

```toml
{{ #include ../../examples/testing/config/test/test_container.toml}}
```

## Docs.rs links

- [`run_test`](https://docs.rs/roadster/0.7.0-beta/roadster/app/fn.run_test.html)
- [`run_test_with_result`](https://docs.rs/roadster/0.7.0-beta/roadster/app/fn.run_test_with_result.html)
- [`snapshot` mod (`insta` utilities)](https://docs.rs/roadster/latest/roadster/testing/snapshot/index.html)
- [Temporary test DB config](https://docs.rs/roadster/0.7.0-beta/roadster/config/database/struct.Database.html#structfield.temporary_test_db)
- [Database TestContainers config](https://docs.rs/roadster/latest/roadster/config/database/struct.Database.html#structfield.test_container)
- [Sidekiq TestContainers config](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.Redis.html#structfield.test_container)
- [SeaORM MockDatabase example](https://github.com/roadster-rs/roadster/blob/df7cd821021a63766eb4e902e6025efaabe95177/examples/full/src/model/user.rs#L36-L42)
