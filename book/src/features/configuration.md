# Configuration

Roadster provides sensible defaults, but is highly customizable via configuration files and environment variables.
Virtually all behavior of Roadster can be configured; to see the available configuration keys, see
the [`AppConfig`](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html) struct.

To see the full app config that's loaded for your app, use the `print-config` CLI command. This will print the app
config after all the config sources (files, env vars, cli args) have been loaded and merged.

```shell
cargo run -- roadster print-config -f toml
```

## Configuration files

The primary way to customize the app's settings is via configuration files. Roadster supports Toml config files by
default, and supports Yaml config files with the `config-yml` feature flag. By default, Roadster looks for config files
in the `config` directory of your project (at the same level as your `Cargo.toml`). However, if you have the `cli`
feature flag enabled, you can specify a different config directory using the `--config-dir` CLI parameter.

You can provide a default configuration and override the defaults for each environment stage. Config files can either
be in a file named `default` or the environment name, or in a directory matching the environment name. Example config
file structure:

```text
my-app/
└── config/
    ├── default.toml
    ├── development.toml
    ├── test.toml
    ├── production.toml
    ├── default/
    │   └── db.toml
    ├── development/
    │   └── db.toml
    ├── test/
    │   └── db.toml
    └── production/
        └── db.toml
```

If there are multiple files in an environment's directory, they are loaded into the config in lexicographical order, and
the last file loaded takes precedence if there are duplicate fields in the files. For example, if an environment
directory contains the files `a.toml` and `b.toml`, `a.toml` is loaded first and `b.toml` is loaded second, and any
duplicate fields in `b.toml` will override the values from `a.toml`.

## Environment variables

You can set environment variables to customize fields. This is useful to provide values for sensitive fields such as DB
passwords. Note that setting passwords as env vars does come with some amount of security risk as they are readable by
anyone who has access to your server, but they're better than checking your sensitive values into git or other source
control.

Env vars can either be set on the command line, or in a `.env` file. Environment variables should be prefixed with
`ROADSTER__` named according to the [`AppConfig`](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html)
structure, where each level of the structure is separated by a double underscore (`__`). For example, to override the
`AppConfig#environment`, you would use an environment variable named `ROADSTER__ENVIRONMENT`, and to override
`AppConfig#app#shutdown_on_error`, you would use `ROADSTER__APP__SHUTDOWN_ON_ERROR`. E.g.:

```shell
export ROADSTER__ENVIRONMENT=dev
export ROADSTER__APP__SHUTDOWN_ON_ERROR=true
```

## Custom Sources

You can also provide one or more [`Source`](https://docs.rs/config/latest/config/trait.Source.html)s to add to
the configuration. This is primarilty intended to allow overriding specific app config fields for tests, but it can also
be used to provide other custom config sources outside of tests.

```rust,ignore
{{#include ../../examples/app-config/src/example_source.rs:6:}}
```

## Custom Async sources

You can also provide one or more [`AsyncSource`](https://docs.rs/config/latest/config/trait.AsyncSource.html)s to add to
the configuration. This is useful to load configuration fields (particularly sensitive ones) from an external service,
such as AWS or GCS secrets manager services.

`AsyncSource`s are loaded into the configuration after all the other sources, so they have the highest precedence (they
will override any duplicate fields from other sources).

```rust,ignore
{{#include ../../examples/app-config/src/example_async_source.rs:6:}}
```

## Config mechanism precedence

- `default.toml` (lowest)
- `default/<filename>.toml`
- `<env>.toml`
- `<env>/<filename>.toml`
- Environment variables
- [`Source`](https://docs.rs/config/latest/config/trait.Source.html)s
- [`AsyncSource`](https://docs.rs/config/latest/config/trait.AsyncSource.html)s (highest -- overrides lower precedence
  values)

If the `config-yml` feature is enabled, files with extensions `.yml` and `.yaml` will be read as well. The precedence of all supported file
extensions is the following:

- `.yml` (lowest)
- `.yaml`
- `.toml` (highest -- overrides lower precedence values)

## Environment names

Roadster provides some pre-defined environment names in
the [`Environment`](https://docs.rs/roadster/latest/roadster/config/environment/enum.Environment.html) enum.

- `development` (alias: `dev`)
- `test`
- `production` (alias: `prod`)

In addition, apps can define a custom environment name, which is mapped to the `Environment::Custom` enum variant. This
is useful for special app-specific environments, such as additional pre-prod or canary environments.

## Docs.rs links

- [`AppConfig`](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html) struct
