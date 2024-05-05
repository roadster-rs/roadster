# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/roadster-rs/roadster/releases/tag/minimal-v0.1.0) - 2024-05-05

### Other
- Upgrade dependencies in `Cargo.toml`s
- Implement the sidekiq processor task as an `AppService`
- Add `ServiceRegistry` and restructure configs
- Add `AppService` trait and use it to implement the HTTP service
- Add example worker and api to the `minimal` example
- Add CLI commands to run DB migrations
- Set up roadster CLI and custom app CLI
- Allow using custom App::State in Initializer and Middleware traits
- Create workspace that includes the examples
- Add a minimal example
