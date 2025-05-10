# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.4...roadster-v0.7.5) - 2025-05-10

### Other

- Pin github actions to specific SHAs ([#752](https://github.com/roadster-rs/roadster/pull/752))
- Run doc checks with `carg hack --each-feature` ([#750](https://github.com/roadster-rs/roadster/pull/750))

## [0.7.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.3...roadster-v0.7.4) - 2025-04-29

### Added

- Set HTTP request span name as `{method} {route}` ([#748](https://github.com/roadster-rs/roadster/pull/748))

## [0.7.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.2...roadster-v0.7.3) - 2025-04-20

### Added

- Use the test name as the test db name ([#741](https://github.com/roadster-rs/roadster/pull/741))
- Remove usages of `anyhow` ([#740](https://github.com/roadster-rs/roadster/pull/740))

## [0.7.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.1...roadster-v0.7.2) - 2025-04-19

### Added

- Add `url.path` and `url.query` trace attributes ([#738](https://github.com/roadster-rs/roadster/pull/738))

## [0.7.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0...roadster-v0.7.1) - 2025-04-15

### Added

- Enable redacting Mysql URIs from `insta` snapshots ([#733](https://github.com/roadster-rs/roadster/pull/733))

## [0.7.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-gamma...roadster-v0.7.0) - 2025-04-13

v0.7.0 is a fairly large internal refactor that introduces a decent number of breaking changes to the public API.
Some notable changes include:

- Add support for Diesel as an alternative to SeaORM
- Add `run_test*` methods to enable running tests against a fully set up app
- Replace the `M` migrator type parameter for the `App` trait with a method to provide the migrator
- API changes to the `RunCommand` trait and the `AppContext` struct
- Update to rust 2024 edition in order to use `AsyncFn`
- Allow either http or grpc OTLP endpoints
- Add support for custom config sources, including async config sources
- Enable fetching a concrete `AppService` from the `ServiceRegistry`

See the changelog for the 0.7.0-* pre-release versions for more details and the full list of breaking changes.
All of the examples are updated to 0.7.0 as well if a reference for how to use 0.7.0 is needed.

## [0.7.0-gamma](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta.5...roadster-v0.7.0-gamma) - 2025-04-10

### Added

- [**breaking**] Enable running app cleanup if test closure
  panics ([#722](https://github.com/roadster-rs/roadster/pull/722))

## [0.7.0-beta.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta.4...roadster-v0.7.0-beta.5) - 2025-04-05

### Added

- [**breaking**] Pad snapshot case number with leading zeros in file
  name ([#718](https://github.com/roadster-rs/roadster/pull/718))

## [0.7.0-beta.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta.3...roadster-v0.7.0-beta.4) - 2025-03-28

### Added

- [**breaking**] Follow OTEL conventions for http spans and
  events ([#710](https://github.com/roadster-rs/roadster/pull/710))
- Allow providing trace env filter directives in app config ([#706](https://github.com/roadster-rs/roadster/pull/706))
- Enable "head sampling" for OTEL traces ([#703](https://github.com/roadster-rs/roadster/pull/703))

### Other

- [**breaking**] Mark tracing middleware structs
  non-exhaustive ([#713](https://github.com/roadster-rs/roadster/pull/713))
- Update dependencies ([#708](https://github.com/roadster-rs/roadster/pull/708))

## [0.7.0-beta.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta.2...roadster-v0.7.0-beta.3) - 2025-03-23

### Added

- [**breaking**] Allow either http or grpc OTLP endpoints ([#701](https://github.com/roadster-rs/roadster/pull/701))

## [0.7.0-beta.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta.1...roadster-v0.7.0-beta.2) - 2025-03-15

### Fixed

- Make `AnyInitializer#stage` field non-optional ([#676](https://github.com/roadster-rs/roadster/pull/676))

### Other

- Update leptos-ssr example to leptos-0.8 and update various
  dependencies ([#689](https://github.com/roadster-rs/roadster/pull/689))
- Various updates to the documentation + book + book examples

## [0.7.0-beta.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-beta...roadster-v0.7.0-beta.1) - 2025-03-04

### Added

- Allow logging sensitive headers in the dev environment ([#666](https://github.com/roadster-rs/roadster/pull/666))
- Allow overriding config fields or entire config ([#661](https://github.com/roadster-rs/roadster/pull/661))
- Accept `Into<String>` for `ConfigOverrideSource` builder ([#670](https://github.com/roadster-rs/roadster/pull/670))

### Fixed

- Export `TestAppState` to allow for external use ([#672](https://github.com/roadster-rs/roadster/pull/672))

### Other

- Update to rust 2024 edition + rustfmt 2024 style edition ([#662](https://github.com/roadster-rs/roadster/pull/662))

## [0.7.0-beta](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.8...roadster-v0.7.0-beta) - 2025-02-25

This is the first beta release for version 0.7.0. From here until the stable 0.7.0 release, the focus will be on
improving docs and internal clean up. Semver breaking changes are not expected going forward for 0.7.0, but are still
possible.

### Other

- Update OTEL dependencies (#659)

## [0.7.0-alpha.8](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.7...roadster-v0.7.0-alpha.8) - 2025-02-25

### Added

- [**breaking**] Allow customizing the Diesel pool connections (#656)

## [0.7.0-alpha.7](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.6...roadster-v0.7.0-alpha.7) - 2025-02-25

### Other

- [**breaking**] Refactor app runners and change command + lifecycle params (#652)
- minor re-word in db book chapter (#651)

## [0.7.0-alpha.6](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.5...roadster-v0.7.0-alpha.6) - 2025-02-24

### Added

- Create temporary databases for tests (#645)
- [**breaking**] Add test hook (#643)

### Fixed

- [**breaking**] Fix `run_test*` to skip CLI (#647)

### Other

- Fix powerset checks (#648)

## [0.7.0-alpha.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.4...roadster-v0.7.0-alpha.5) - 2025-02-20

### Added

- Support mysql test container (#636)

### Fixed

- Use `db-sql` instead of `db-sea-orm` where appropriate (#637)

### Other

- Remove unnecessary `Empty` for `RoadsterApp` `Cli` type (#633)
- Add `diesel` to `loco` comparison page (#632)
- Update db chapter of book (#631)

## [0.7.0-alpha.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.3...roadster-v0.7.0-alpha.4) - 2025-02-18

This is a very large release with a lot of breaking changes. See the below changelog the detailed commit history. In
summary, this release adds support for the [Diesel](https://github.com/diesel-rs/diesel) SQL ORM. Diesel is a very
different ORM compared to SeaORM (the ORM we currently support), and as such this release required a lot of refactoring
in order to provide a relatively consistent experience regardless of which ORM a consumer decides to use (or if they
decide to use both, which is possible but not particularly recommended). The refactor also resulted in some general
simplifications and improvements to the devx; read on for more details. Some breaking changes include:

- Remove the `M` associated type from the `App` trait. A `Migrator` can now be provided via the `migrators` method
  instead.
- Similarly, remove the `M` type parameter from `RoadsterApp`. SeaORM, Diesel, or a generic `Migrator` can now be
  provided via the builder methods.
- Change `RunCommand#run` to take a single `PreparedApp` struct
- ^This allowed removing the CLI handler method from the `AppService` trait. CLI's now have access to the
  `ServiceRegistry` from the `PreparedApp`, so they can get access to a particular `AppService` using
  `ServiceRegistry#get` (assuming it was registered).
- Consolidate DB migration CLI commands to provide consistent experience between SeaORM and Diesel. This also removed
  some slightly redundant commands.
- Rename  `AppContext#db` to `AppContext#sea_orm`
- Rename `App#db_connection_options` to
  `App#sea_orm_connection_options`, and rename the related methods in `RoadsterApp`
- Move/rename the DB health check
- Add `Sized` as a parent trait for the `App`

This release also includes the following non-breaking changes:

- Add `AppContext` methods to get various Diesel connection pools types, including Postgres, Mysql, Sqlite, and async
  pools for Postgres and Mysql. Due to Diesel's type strategy for connections, there isn't a single "DbConnection" like
  there is in SeaORM, so we provide individual methods depending on which feature flags are enabled.
- Allow providing `AsyncSource` implementations to use with the `config` crate. This allows, for example, loading secret
  config values from an external service, such as AWS or GCS secrets managers.
- Add a couple db config fields, `test-on-checkout` and `retry-connection`
- Add more variants to our custom `Error` type

### Added

- [**breaking**] Add `diesel` support (#626)
- [**breaking**] Add `db-sea-orm` feature to prepare for other DB crate support (#612)

### Fixed

- Kebab case for environment env var instead of lowercase (#614)

### Other

- [**breaking**] Replace native-tls with rustls in several dependencies (#621) thanks to @tomtom5152
- [**breaking**] Remove `AppContext::mailer` method in favor of the `smtp` method (#613)
- Remove leptos-0.6 example so to maintain a single leptos example (#610)
- Add doc comments for `Provide` and `ProvideRef` and add to book (#598)
- Minor improvement to initializing health checks in state (#593)
- Refactor `RoadsterApp` to reduce duplication (#589)
- Add example of using tower/axum `oneshot` to test APIs (#587)
- Improve test coverage (#582)
- Update the validator trait (#585)
- Various documentation + test improvements

## [0.7.0-alpha.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.2...roadster-v0.7.0-alpha.3) - 2025-01-22

### Added

- [**breaking**] Enable fetching concrete service from registry via downcast (#580)

### Other

- [**breaking**] Rename/move the `health_check` mod to `health::check` (#578)
- [**breaking**] Remove the `App#graceful_shutdown` method (#577)
- Add `ExampleHealthCheck` to the `full` example (#576)

## [0.7.0-alpha.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha.1...roadster-v0.7.0-alpha.2) - 2025-01-18

### Added

- Map `Error::Auth` to `StatusCode::UNAUTHORIZED` HTTP response (#571)
- [**breaking**] Return `RedisEnqueue` and `RedisFetch` redis pool "new-types" (#568)

### Other

- Remove todos (#570)
- [**breaking**] Remove `From<Environment>` impl for `&'static str` (#569)
- Declare all dependencies in workspace (#567)
- [**breaking**] Remove deprecated items (#566)

## [0.7.0-alpha.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.7.0-alpha...roadster-v0.7.0-alpha.1) - 2025-01-17

### Added

- [**breaking**] Allow registering `Worker` instead of requiring `AppWorker` (#564)
- Increase the default cache-control max-age to 1 week (#559)

### Fixed

- Use `Router#fallback_service` in `NormalizePathInitializer` (#562)

### Other

- Update `config` to 0.15.6 (#560)

## [0.7.0-alpha](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.24...roadster-v0.7.0-alpha) - 2025-01-14

### Added

- Support sidekiq balance strategy and dedicated queues (#543)

### Fixed

- [**breaking**] Propagate `Validate` calls to all app config fields (#557)

### Other

- [**breaking**] Update bb8 to v0.9 and sidekiq-rs to 0.13.1 (#555)
- Add some details to config book page (#553)
- Upgrade various crates that are used internally (#551)
- [**breaking**] Upgrade Axum to 0.8 and Aide to 0.14 (#548)
- Enable nightly coverage feature (#545)

## [0.6.24](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.23...roadster-v0.6.24) - 2024-12-28

### Added

- Add cache-related middlewares (#541)
- Add timestamps for when email change is confirmed (#537)
- Add User column to store the user's new email before it's confirmed (#536)

## [0.6.23](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.22...roadster-v0.6.23) - 2024-12-24

### Added

- Enable redacting timestamps from `insta` snapshots (#532)
- Add `AppWorker#enqueue_delayed` (#531)

## [0.6.22](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.21...roadster-v0.6.22) - 2024-12-07

### Added

- Add `AppContextWeak` to prevent reference cycles ([#529](https://github.com/roadster-rs/roadster/pull/529))

## [0.6.21](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.20...roadster-v0.6.21) - 2024-12-01

### Other

- Update OTEL patch version and remove a deprecated fn call ([#527](https://github.com/roadster-rs/roadster/pull/527))
- Update Loco comparisons and add some links to other
  sections ([#522](https://github.com/roadster-rs/roadster/pull/522))
- Add mailpit to SMTP dev server examples ([#521](https://github.com/roadster-rs/roadster/pull/521))
- *(deps)* bump codecov/codecov-action from 4 to 5 ([#517](https://github.com/roadster-rs/roadster/pull/517))
- Upgrade otel/tracing dependencies ([#516](https://github.com/roadster-rs/roadster/pull/516))

## [0.6.20](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.19...roadster-v0.6.20) - 2024-11-17

### Added

- Enable converting `roadster::Error` to `sidekiq::Error` ([#514](https://github.com/roadster-rs/roadster/pull/514))

### Other

- Use `MockProvideRef<DatabaseConnection>` in an example test ([#513](https://github.com/roadster-rs/roadster/pull/513))

## [0.6.19](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.18...roadster-v0.6.19) - 2024-11-16

### Added

- `Provide` and `ProvideRef` traits to provide `AppContext`
  objects ([#510](https://github.com/roadster-rs/roadster/pull/510))

## [0.6.18](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.17...roadster-v0.6.18) - 2024-11-15

### Added

- Add support for redacting postgres/redis/smtp URIs ([#507](https://github.com/roadster-rs/roadster/pull/507))

### Other

- Add `smtp4dev` to example local SMTP servers ([#506](https://github.com/roadster-rs/roadster/pull/506))

## [0.6.17](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.16...roadster-v0.6.17) - 2024-11-12

### Added

- Add support for TestContainers (pgsql + redis modules) ([#503](https://github.com/roadster-rs/roadster/pull/503))

### Other

- Update `thiserror` to 2.x ([#499](https://github.com/roadster-rs/roadster/pull/499))
- Update `validator` crate ([#497](https://github.com/roadster-rs/roadster/pull/497))

## [0.6.16](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.15...roadster-v0.6.16) - 2024-10-28

### Added

- Add config to specify the domain where the service is
  hosted ([#490](https://github.com/roadster-rs/roadster/pull/490))

## [0.6.15](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.14...roadster-v0.6.15) - 2024-10-22

### Added

- Case-insensitive username and email fields ([#480](https://github.com/roadster-rs/roadster/pull/480))

## [0.6.14](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.13...roadster-v0.6.14) - 2024-10-21

### Added

- Add `AnyMiddleware` to minimize boilerplate for Axum
  middleware ([#472](https://github.com/roadster-rs/roadster/pull/472))
- Add `AnyIntializer` to minimize boilerplate for Axum Router
  initializers ([#475](https://github.com/roadster-rs/roadster/pull/475))

### Other

- Add leptos-0.7 example ([#465](https://github.com/roadster-rs/roadster/pull/465))

## [0.6.13](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.12...roadster-v0.6.13) - 2024-10-19

### Fixed

- Only attempt to load yaml files when `config-yaml` is
  enabled ([#451](https://github.com/roadster-rs/roadster/pull/451))

### Other

- Use `FromRef` from `axum-core` instead of `axum` ([#450](https://github.com/roadster-rs/roadster/pull/450))

## [0.6.12](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.11...roadster-v0.6.12) - 2024-10-17

### Added

- Enable writing config files in YAML ([#446](https://github.com/roadster-rs/roadster/pull/446))

## [0.6.11](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.10...roadster-v0.6.11) - 2024-10-17

### Fixed

- Fix trace message for health checks on startup ([#443](https://github.com/roadster-rs/roadster/pull/443))

### Other

- Add loco comparison ([#444](https://github.com/roadster-rs/roadster/pull/444))

## [0.6.10](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.9...roadster-v0.6.10) - 2024-10-16

### Added

- Enable consumers to provide custom Environment values ([#439](https://github.com/roadster-rs/roadster/pull/439))

## [0.6.9](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.8...roadster-v0.6.9) - 2024-10-15

### Added

- Add `AppContext::smtp` method to alias to
  `AppContext::mailer` ([#409](https://github.com/roadster-rs/roadster/pull/409))
- Create documentation website using [mdbook](https://github.com/rust-lang/mdBook). The website can be found
  at [roadster.dev](https://roadster.dev).

### Other

- Update sea-orm ([#434](https://github.com/roadster-rs/roadster/pull/434))
- Create SECURITY.md ([#420](https://github.com/roadster-rs/roadster/pull/420))
- Create CODE_OF_CONDUCT.md ([#419](https://github.com/roadster-rs/roadster/pull/419))

## [0.6.8](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.7...roadster-v0.6.8) - 2024-10-11

### Added

- Allow configuring which req/res body content types to log ([#407](https://github.com/roadster-rs/roadster/pull/407))

## [0.6.7](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.6...roadster-v0.6.7) - 2024-10-11

### Added

The main feature included in this release is support for sending emails
via [Sendgrid's Mail Send API](https://www.twilio.com/docs/sendgrid/api-reference/mail-send/mail-send). See the below
items for more details.

- Set sandbox mode on Sendgrid message based on config ([#403](https://github.com/roadster-rs/roadster/pull/403))
- Add Sendgrid client to `AppContext` ([#402](https://github.com/roadster-rs/roadster/pull/402))
- Add support to config for email via Sendgrid (`email-sendgrid`
  feature) ([#401](https://github.com/roadster-rs/roadster/pull/401))

### Other

- Add note to readme about supporting Sendgrid ([#405](https://github.com/roadster-rs/roadster/pull/405))
- Add example of using Sendgrid client ([#404](https://github.com/roadster-rs/roadster/pull/404))

## [0.6.6](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.5...roadster-v0.6.6) - 2024-10-10

### Added

- Allow configuring the interval at which metrics are
  exported ([#399](https://github.com/roadster-rs/roadster/pull/399))

## [0.6.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.4...roadster-v0.6.5) - 2024-10-09

### Added

The main feature included in this release is support for sending emails via SMTP. See the below items for more details.

- Add `SmtpHealthCheck` ([#396](https://github.com/roadster-rs/roadster/pull/396))
- Allow specifying the smtp port via config ([#395](https://github.com/roadster-rs/roadster/pull/395))
- Add smtp client to `AppContext` ([#391](https://github.com/roadster-rs/roadster/pull/391))
- Add support to config for email via SMTP (`email-smtp`
  feature) ([#388](https://github.com/roadster-rs/roadster/pull/388))

### Fixed

- Fix config value used for timeout of health check in api and
  cli ([#397](https://github.com/roadster-rs/roadster/pull/397))

### Other

- Add example of sending email using lettre smtp client ([#394](https://github.com/roadster-rs/roadster/pull/394))
- Add doc comment explaining how NormalizePathLayer works ([#393](https://github.com/roadster-rs/roadster/pull/393))

## [0.6.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.3...roadster-v0.6.4) - 2024-10-05

### Other

- Update dependencies ([#386](https://github.com/roadster-rs/roadster/pull/386))
- Disable default features for `rstest` ([#380](https://github.com/roadster-rs/roadster/pull/380))

## [0.6.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.2...roadster-v0.6.3) - 2024-09-15

### Added

- Add more builder methods for `RoadsterApp` ([#370](https://github.com/roadster-rs/roadster/pull/370))
- Builder-style API for `App` ([#367](https://github.com/roadster-rs/roadster/pull/367))

### Other

- Add logs for successful health checks ([#371](https://github.com/roadster-rs/roadster/pull/371))

## [0.6.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.1...roadster-v0.6.2) - 2024-08-30

### Added

- Allow specifying a custom config dir ([#361](https://github.com/roadster-rs/roadster/pull/361))
- Add lifecycle handlers ([#360](https://github.com/roadster-rs/roadster/pull/360))

## [0.6.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.6.0...roadster-v0.6.1) - 2024-08-28

### Added

- Allow running CLI commands without requiring DB/Redis
  connections ([#353](https://github.com/roadster-rs/roadster/pull/353))

### Other

- Update `typed-builder` and several examples' dependencies ([#352](https://github.com/roadster-rs/roadster/pull/352))

## [0.6.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.19...roadster-v0.6.0) - 2024-08-25

### Added

- Add a public method to decode a JWT from a string ([#348](https://github.com/roadster-rs/roadster/pull/348))
- Mark refresh token headers as sensitive ([#347](https://github.com/roadster-rs/roadster/pull/347))
- Make the `User` sea-orm migration enum public ([#346](https://github.com/roadster-rs/roadster/pull/346))
- Allow splitting config files into many files in env
  directories ([#344](https://github.com/roadster-rs/roadster/pull/344))
- [**breaking**] App methods take `self` ([#337](https://github.com/roadster-rs/roadster/pull/337))
- Remove cookie extraction for `Jwt`, but allow it in
  `JwtCsrf` ([#332](https://github.com/roadster-rs/roadster/pull/332))
- Allow custom sub-claims in provided `Claims` types ([#331](https://github.com/roadster-rs/roadster/pull/331))
- Allow jwt from cookie, but only if it's explicitly
  requested ([#329](https://github.com/roadster-rs/roadster/pull/329))

### Fixed

- [**breaking**] Don't expect a "Bearer" token in the auth token
  cookie ([#340](https://github.com/roadster-rs/roadster/pull/340))

### Other

- Update leptos example to use site-addr and env from roadster
  config ([#341](https://github.com/roadster-rs/roadster/pull/341))
- sea-orm workspace dep and upgrade to `1.0.0` ([#336](https://github.com/roadster-rs/roadster/pull/336))
- [**breaking**] Update tower to `0.5.0` ([#334](https://github.com/roadster-rs/roadster/pull/334))

## [0.5.19](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.18...roadster-v0.5.19) - 2024-08-12

### Added

- Redact bearer tokens in insta snapshots ([#325](https://github.com/roadster-rs/roadster/pull/325))

### Fixed

- Do not simply use bearer token from cookie for auth ([#326](https://github.com/roadster-rs/roadster/pull/326))
- Derive `Clone` in JWT claim types ([#323](https://github.com/roadster-rs/roadster/pull/323))
- Implement `From` for various `Subject` enum variants ([#323](https://github.com/roadster-rs/roadster/pull/323))
- Use `leptos_routes` in leptos example instead of
  `leptos_routes_with_context` ([#322](https://github.com/roadster-rs/roadster/pull/322))

### Other

- *(deps)* Bump EmbarkStudios/cargo-deny-action from 1 to 2 ([#319](https://github.com/roadster-rs/roadster/pull/319))

## [0.5.18](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.17...roadster-v0.5.18) - 2024-08-05

### Other

- Update `rstest` dependency ([#318](https://github.com/roadster-rs/roadster/pull/318))

## [0.5.17](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.16...roadster-v0.5.17) - 2024-08-05

### Fixed

- Extract jwt as a bearer token from cookies ([#316](https://github.com/roadster-rs/roadster/pull/316))

## [0.5.16](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.15...roadster-v0.5.16) - 2024-08-04

### Added

- Extract JWT from cookie ([#314](https://github.com/roadster-rs/roadster/pull/314))
- Derive `OperationIo` for `Jwt` struct ([#311](https://github.com/roadster-rs/roadster/pull/311))
- Change user.last_sign_in_at column to non-null with default ([#312](https://github.com/roadster-rs/roadster/pull/312))

### Other

- Add pre-commit hook to check formatting ([#313](https://github.com/roadster-rs/roadster/pull/313))

## [0.5.15](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.14...roadster-v0.5.15) - 2024-08-01

### Added

- Allow configuring the max len for the
  `ReqResLoggingMiddleware` ([#309](https://github.com/roadster-rs/roadster/pull/309))

## [0.5.14](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.13...roadster-v0.5.14) - 2024-08-01

### Added

- Enable ReqResLogging middleware by default, but disable in
  prod ([#307](https://github.com/roadster-rs/roadster/pull/307))

## [0.5.13](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.12...roadster-v0.5.13) - 2024-07-31

### Added

- Add middleware to log the request/response payloads ([#304](https://github.com/roadster-rs/roadster/pull/304))
- Log errors at debug level in `IntoResponse` impl ([#303](https://github.com/roadster-rs/roadster/pull/303))

## [0.5.12](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.11...roadster-v0.5.12) - 2024-07-29

### Added

- PasswordUpdatedAt column + auto-update with a fn and
  trigger ([#301](https://github.com/roadster-rs/roadster/pull/301))

## [0.5.11](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.10...roadster-v0.5.11) - 2024-07-26

### Added

- Migration to enable the uuid-ossp Postgres extension ([#297](https://github.com/roadster-rs/roadster/pull/297))
- Add non-pk versions of uuid schema helper methods ([#296](https://github.com/roadster-rs/roadster/pull/296))

## [0.5.10](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.9...roadster-v0.5.10) - 2024-07-25

### Added

- Use IDENTITY column for int primary keys instead of
  BIGSERIAL ([#293](https://github.com/roadster-rs/roadster/pull/293))

### Fixed

- Add "if exists" to user's drop_table migration statement ([#292](https://github.com/roadster-rs/roadster/pull/292))

### Other

- Add tests for schema and check helper methods ([#289](https://github.com/roadster-rs/roadster/pull/289))

## [0.5.9](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.8...roadster-v0.5.9) - 2024-07-24

### Added

- Auto-update timestamp columns ([#287](https://github.com/roadster-rs/roadster/pull/287))
- Add SeaORM migrations and utils to create `user` table ([#284](https://github.com/roadster-rs/roadster/pull/284))

### Other

- Disallow `unwrap` and `expect` except in tests ([#286](https://github.com/roadster-rs/roadster/pull/286))

## [0.5.8](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.7...roadster-v0.5.8) - 2024-07-22

### Other

- Remove the `update` justfile command ([#282](https://github.com/roadster-rs/roadster/pull/282))
- Use the main project README.md as the library's top-level
  docs ([#281](https://github.com/roadster-rs/roadster/pull/281))

## [0.5.7](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.6...roadster-v0.5.7) - 2024-07-22

### Other

- Update dependencies ([#279](https://github.com/roadster-rs/roadster/pull/279))

## [0.5.6](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.5...roadster-v0.5.6) - 2024-07-22

### Added

- Add `TestCase` utility for configuring `insta` settings ([#277](https://github.com/roadster-rs/roadster/pull/277))

## [0.5.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.4...roadster-v0.5.5) - 2024-07-08

### Added

- Allow configuring the tracing log output format ([#275](https://github.com/roadster-rs/roadster/pull/275))

## [0.5.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.3...roadster-v0.5.4) - 2024-07-07

### Added

- Add method to prepare the app separately from running it ([#270](https://github.com/roadster-rs/roadster/pull/270))

### Fixed

- Correctly add the `ApiRouter` to the HTTP service's
  `ApiRouter` ([#273](https://github.com/roadster-rs/roadster/pull/273))

### Other

- Fixes for default openapi docs ([#271](https://github.com/roadster-rs/roadster/pull/271))

## [0.5.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.2...roadster-v0.5.3) - 2024-07-04

### Other

- Update the `_health` HTTP API docs ([#267](https://github.com/roadster-rs/roadster/pull/267))

## [0.5.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.1...roadster-v0.5.2) - 2024-07-02

### Added

- Allow configuring the max duration of health checks ([#264](https://github.com/roadster-rs/roadster/pull/264))

## [0.5.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.5.0...roadster-v0.5.1) - 2024-07-02

### Added

- Place health check results under `resources` in response ([#261](https://github.com/roadster-rs/roadster/pull/261))

### Other

- Fix typos in README ([#260](https://github.com/roadster-rs/roadster/pull/260))

## [0.5.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.4.0...roadster-v0.5.0) - 2024-07-01

### Added

- [**breaking**] Remove interior mutability of
  `HealthCheckRegistry` ([#258](https://github.com/roadster-rs/roadster/pull/258))

## [0.4.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.5...roadster-v0.4.0) - 2024-07-01

### Added

- [**breaking**] Implement health check API using `HealthCheck`
  trait ([#255](https://github.com/roadster-rs/roadster/pull/255))
- [**breaking**] Switch to Axum's `FromRef` for custom state ([#250](https://github.com/roadster-rs/roadster/pull/250))

### Other

- [**breaking**] Remove deprecated items in preparation of 0.4
  release ([#253](https://github.com/roadster-rs/roadster/pull/253))
- Add example for integrating with Leptos ([#252](https://github.com/roadster-rs/roadster/pull/252))
- Use small number of sidekiq workers for `full` example in
  dev/test ([#251](https://github.com/roadster-rs/roadster/pull/251))

## [0.3.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.4...roadster-v0.3.5) - 2024-06-24

### Fixed

- Health check config is missing a `custom` field ([#246](https://github.com/roadster-rs/roadster/pull/246))

### Other

- Check PR title for compliance with conventional commits ([#247](https://github.com/roadster-rs/roadster/pull/247))

## [0.3.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.3...roadster-v0.3.4) - 2024-06-23

### Added

- Add health checks to run before starting services ([#242](https://github.com/roadster-rs/roadster/pull/242))
- Add `From` impl to convert db config to ConnectOptions ([#240](https://github.com/roadster-rs/roadster/pull/240))
- Move sidekiq "stale cleanup" to new `before_run` service
  method ([#239](https://github.com/roadster-rs/roadster/pull/239))

### Other

- Add dependabot config to update github actions weekly ([#243](https://github.com/roadster-rs/roadster/pull/243))
- Update READMEs to use `__` as the env var separator instead of `.`
- Update list of UI frameworks in readme
- Set up `cargo deny`

## [0.3.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.2...roadster-v0.3.3) - 2024-06-21

### Fixed

- Invalid env var separator on bash

### Other

- Add inclusive language check to CI
- Fix clippy error
- Remove non-inclusive language

## [0.3.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.1...roadster-v0.3.2) - 2024-06-14

### Other

- Run Feature Powerset checks + perform a release twice a week
- Add goals and future plans to readme + some getting started steps
- Add github action to verify commits follow 'Conventional Commits' format

## [0.3.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.0...roadster-v0.3.1) - 2024-06-11

### Other

- Implement the health check API as a protocol agnostic `core` module
- Minor changes to the `FunctionService` doc example

## [0.3.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.6...roadster-v0.3.0) - 2024-06-10

### Other

- Fix minimal version of serde
- Add #[non_exhaustive] to public enums
- Add Add #[non_exhaustive] to public structs
- Enable grpc by default in the `full` example
- Add support for tower's CORS middleware
- Add AppMetadata struct + App::metadata method and add version to otel
- Run doctests as part of test and test-watch just commands
- Update readme to include grpc and generic function service
- Update FunctionService doctest to only run with default features
- Add a generic app service to run an async function as a service
- Move semver checks to a separate workflow
- Use depth 3 in feature powerset
- Install protoc in feature powerset workflow
- Remove the old deprecated cli mod
- Add basic grpc example
- Add basic support for serving a gRPC service
- Update rstest

## [0.2.6](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.5...roadster-v0.2.6) - 2024-06-03

### Other

- Add builder method to add middleware for the sidekiq processor
- Declare minimal version of dependencies that's actually needed
- Add `cargo-minimal-versions` for direct dependencies

## [0.2.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.4...roadster-v0.3.0) - 2024-06-03

### Other

- Only test the `AppConfig#test` method when all (most) features are enabled
- Hard-code the number of sidekiq workers to avoid snapshot failures
- Ignore a clippy error
- Ignore coverage for the `AppConfig#test` method
- Provide config defaults via config files
- Move database and tracing mods to directories
- Upgrade dependencies

## [0.2.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.3...roadster-v0.2.4) - 2024-05-31

### Other

- Upgrade dependencies

## [0.2.3](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.2...roadster-v0.2.3) - 2024-05-27

### Other

- Remove `http` feature gate for `api` mod
- Move cli mod to be a child of the api mod
- Add semver checks to CI
- Revert "Use stable rust for coverage"

## [0.2.2](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.1...roadster-v0.2.2) - 2024-05-26

### Other

- Add latest version of `time` to workaround build issue on nightly

## [0.2.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.2.0...roadster-v0.2.1) - 2024-05-26

### Other

- Add missing `needs` field to `powerset_clippy` workflow step
- Run separate jobs for each feature powerset check

## [0.2.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.1.1...roadster-v0.2.0) - 2024-05-26

### Other

- Add custom error type using `thiserror`
- Fix incorrect feature flag used on import statement
- Allow partial overrides of all configs
- Remove mock `AppContext` and use a concrete version in tests instead
- Add a small description for the `validate-codecov-config` just cmd
- Use automock to mock traits instead of the mock! macro
- Use github discussions instead of discord (for now at least)
- Add tests for controller config methods
- Fix exiting the app when a cli command is handled
- Allow running the release_pr workflow manually
- Add code owners to automatically request reviews on PRs
- Add documentation for the TestCase utility class
- Use stable rust for coverage
- Add tests for sidekiq builder and roadster cli
- Run release pr workflow with manual dispatch
- Separate different parts of the app::start method into respective mods
- Add tests for the `DefaultRoutes` config validator
- Fix `DefaultRoutes` validator when open-api feature is not enabled
- Add validation of the AppConfig
- Add deps.rs badge to readme
- Group `http` and `open-api` features in the feature_powerset workflow
- Update the codecov PR comment config
- Add tests for `remove_stale_periodic_jobs`
- Update dependencies that can be updated
- Add a feature flag to entirely disable the http service
- Add comments to justfile
- Update dependencies and add `just` command to update deps
- Add MSRV tag to readme
- Add MSRV and add CI step to validate
- Add some tests for `SidekiqWorkerServiceBuilder`
- Add justfile
- Add tests for `SidekiqWorkerService::enabled`
- Fix clippy warning
- Allow unknown cfg in coverage workflow
- Disable coverage for tests
- Enable running coverage using the nightly toolchain
- Add tests for the Middleware::priority methods for each middleware
- Add tests for the Middleware::enabled method for each middleware
- Add tests for the default_middleware and default_initializers methods
- Add tests for middleware/initializer registration in HttpServiceBuilder
- Don’t use coverage(off) for now because it’s unstable
- Use coverage instead of coverage_nightly
- Apply coverage(off) directly to the desired method
- Use coverage(off) only with cfg_attr(coverage)
- Disable coverage for service mod test impls
- Add small test for service builder
- Remove async-std from dev deps
- Remove Tokio from non-async test
- Use Tokio for rstest tests
- Run `cargo upgrade` to update dependencies
- Update the code coverage comment format
- Set up mocking using `mockall` crate
- Do some test cleanup
- Add test for route that isn't documented
- Add test for the HttpService::list_routes method/cli command
- Rename the custom context in the minimal example
- Remove unnecessary `From...` impl for `AppContext`
- Pass config and context by reference in all public APIs
- Custom state as member of `AppContext`
- Add methods to AppContext instead of direct field access
- Fix codecov config
- Add tests to serde_util
- Add codecov config file
- Update instructions to run CI locally
- Add coverage badge to the readme
- Add workflow to generate code coverage stats
- Disallow registering things multiple times
- Create FUNDING.yml
- Update feature_powerset.yml schedule
- Rearrange and enhance the status badges in the readme
- Add a Discord badge
- Have docs.rs pass --all-features to ensure all features have docs built

## [0.1.1](https://github.com/roadster-rs/roadster/compare/roadster-v0.1.0...roadster-v0.1.1) - 2024-05-05

### Other

- Only run the release workflow on main
- Install missing nextest dependency for feature powerset
- Add crates.io and docs.rs badges
- Run the release workflow after the `Feature Powerset` workflow succeeds

## [0.1.0](https://github.com/roadster-rs/roadster/releases/tag/roadster-v0.1.0) - 2024-05-05

### Other

- Set `publish = false` in the minimal example Cargo.toml
- Set `publish = true` in the Cargo.toml
- Remove fetch depth of 0 from CI and feature powerset
- Automate releases with release-plz
- Use fetch depth of 0 in CI
- Remove `lazy_static` dependency
- Use the nextest test runner
- Upgrade dependencies in `Cargo.toml`s
- Fix the graceful shutdown of the sidekiq service
- Timeout the ping redis method so the health route can return without fully timing out
- Implement the sidekiq processor task as an `AppService`
- Move middleware/initializers to service/http module
- Add `ServiceRegistry` and restructure configs
- Add `AppService` trait and use it to implement the HTTP service
- Add example worker and api to the `minimal` example
- Don't run Processor depending on configs
- Use a separate redis connection pool for enqueuing vs fetching jobs
- Remove `url::Url` import from `app_config.rs` for `sidekiq` feature
- Allow configuring the number of sidekiq worker tasks
- Remove stale periodic jobs
- Enable registering periodic workers
- Check disk usage between feature powerset workflow steps
- Add defaults for `AppWorkerConfig`'s builder
- Add RoadsterWorker to provide common behaviors for workers
- Add instructions for RedisInsight to the readme
- Add standalone sidekiq dashboard instructions to readme
- Clean between powerset build stages
- Skip and group features to reduce powerset size
- Use cfg feature flag instead of allowing unused import
- Add feature flag to enable exporting traces/metrics using otel
- Add to list of features in readme
- Add CLI command to print the app config
- Add CLI commands to run DB migrations
- Add CLI command to generate an openapi schema
- Allow private intra doc links for rustdoc
- Add CLI command to list API routes
- Set up roadster CLI and custom app CLI
- Fix the cron used for `feature_powerset.yml` workflow
- Remove a `cfg` that caused a build error
- Add doc comment for `Initializer::priority`
- Allow using custom App::State in Initializer and Middleware traits
- Remove debugging outputs
- Fix step names used to define outputs
- Add missing runs-on field
- Add debugging log to workflow
- Add log of label name
- Use uniq job output names
- Fix error in feature_powerset.yml
- Allow triggering the feature powerset check by adding a lable to a pr
- Add missing cfg for the `open-api` feature
- Fix a powerset build error
- Add `Swatinem/rust-cache@v2` to cache rust builds
- Update checkout action version
- Add `workflow_dispatch` event to feature_powerset.yml
- Add github workflow to run checks against the powerset of features
- Remove "all features" job b/c it's a duplicate of the cargo hack job
- Use `cargo hack --each-feature` instead of `--feature-powerset`
- Add openid jwt claims
- Minor changes
- Fix build break with all features disabled
- Enable reporting traces/metrics via an otlp exporter
- Use snake case in github ci job
- Add RequestDecompressionMiddleware
- Add more crate-level documentation
- Create LICENSE
- Move workspace declaration to the bottom of the Cargo.toml
- Fix a `rustdoc::all` warning
- Remove `--no-dev-deps` where it can't be used in github ci workflow
- Use `cargo hack` to test feature powerset
- Update cargo checks
- Fix cargo fmt command
- Create workspace that includes the examples
- Set working dir for examples job
- Add a minimal example
- Don't run clippy against deps in ci
- Add CI badge to the readme
- Update checkout action to v4
- Add workflow stage to run checks for all features
- Add missing checkout in workflow
- Use custom husky hooks
- Add github workflow to run checks with all feature combinations
- Add feature flag for generating openapi schema using `aide`
- Add feature flag for the SQL db
- Add feature flag for sidekiq
- Add RequestBodyLimitMiddleware
- Add TimeoutLayer middleware
- Add instructions for generating an html coverage report
- Use `JoinSet` instead of `TaskTracker`
- Make the Jwt claims type generic and use `Claims` as the default
- Add notes on background job queue options
- Add JWT extractor with basic Claims impl for default/recommended claims
- Add logs for sidekiq queues
- Add ping latencies to health check response
- Allow configuring the max number of redis connections
- Don't bail early in graceful shutdown if an error occurred.
- Minor string change
- Remove `instrument` from `cancel_on_error`
- Remove `log` from dependencies
- Improve graceful shutdown logic
- Always run shutdown logic and don't require consumer to run it
- Add token cancelation drop guard, and add doc comment recommending to use the default shutdown signal
- Add logs for installing middleware
- Add compression middleware
- Remove stray log
- Add catch panic middleware
- Add graceful shutdown signal
- Add rusty-sidekiq for running async jobs
- Add `_health` route to check the health of the service
- Enable migrations
- Add SeaORM integration
- Enable custom configs for initializers
- Add Initializer with various hooks, and add NormalizePathInitializer
- Minor change to concat middleware vecs inline
- Don't require consumers to include default middleware
- Reorder default middleware -- order determined by config now
- Enable providing configs for custom middleware
- Enable configuring middleware
- Add environment to the AppConfig
- Add OpenAPI docs + spec routes
- Add tracing middleware
- Add request id middleware
- Allow middleware installers to return a result
- Enable adding middleware and provide defaults
- Require custom state to be convertable to AppContext
- Add default _ping route
- Enable defining routes using Axum or Aide routers
- Use From trait instead of a custom trait
- Re-order dependencies in Cargo.toml
- Add App trait and allow providing a custom state
- Add app entrypoint
- Init tracing
- Add basic configuration support
- Remove .idea directory
- Remove Cargo.lock from git
- Move cargo-husky to dev-deps
- Prevent publishing for now
- Add cargo-husky
- Init and add empty rust lib project
