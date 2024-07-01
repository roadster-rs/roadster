# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.4.0...roadster-v0.5.0) - 2024-07-01

### Added
- [**breaking**] Remove interior mutability of `HealthCheckRegistry` ([#258](https://github.com/roadster-rs/roadster/pull/258))

## [0.4.0](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.5...roadster-v0.4.0) - 2024-07-01

### Added
- [**breaking**] Implement health check API using `HealthCheck` trait ([#255](https://github.com/roadster-rs/roadster/pull/255))
- [**breaking**] Switch to Axum's `FromRef` for custom state ([#250](https://github.com/roadster-rs/roadster/pull/250))

### Other
- [**breaking**] Remove deprecated items in preparation of 0.4 release ([#253](https://github.com/roadster-rs/roadster/pull/253))
- Add example for integrating with Leptos ([#252](https://github.com/roadster-rs/roadster/pull/252))
- Use small number of sidekiq workers for `full` example in dev/test ([#251](https://github.com/roadster-rs/roadster/pull/251))

## [0.3.5](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.4...roadster-v0.3.5) - 2024-06-24

### Fixed
- Health check config is missing a `custom` field ([#246](https://github.com/roadster-rs/roadster/pull/246))

### Other
- Check PR title for compliance with conventional commits ([#247](https://github.com/roadster-rs/roadster/pull/247))

## [0.3.4](https://github.com/roadster-rs/roadster/compare/roadster-v0.3.3...roadster-v0.3.4) - 2024-06-23

### Added
- Add health checks to run before starting services ([#242](https://github.com/roadster-rs/roadster/pull/242))
- Add `From` impl to convert db config to ConnectOptions ([#240](https://github.com/roadster-rs/roadster/pull/240))
- Move sidekiq "stale cleanup" to new `before_run` service method ([#239](https://github.com/roadster-rs/roadster/pull/239))

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
