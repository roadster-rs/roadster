# Configuration

Roadster provides sensible defaults, but is highly customizable via configuration files and environment variables.

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
    ├── production/
    │   └── db.toml
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
`ROADSTER__` named according to the [AppConfig](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html)
structure, where each level of the structure is separated by a double underscore (`__`). For example, to override the
`AppConfig#environment`, you would use an environment variable named `ROADSTER__ENVIRONMENT`, and to override
`AppConfig#app#name`, you would use `ROADSTER__APP__NAME`. E.g.:

```shell
export ROADSTER__ENVIRONMENT=dev
export ROADSTER__APP__NAME='My App'
```

## Config mechanism precedence

- `default.yml` (lowest)
- `default.toml` (lowest)
- `default/<filename>.toml`
- `<env>.toml`
- `<env>/<filename>.toml`
- Environment variables (highest -- overrides lower precedence values)

If the `config-yml` feature is enabled, the precedence of file extensions is the following:

- `.yml`
- `.yaml`
- `.toml`

## Environment names

Roadster provides some pre-defined environment names in
the [Environment](https://docs.rs/roadster/latest/roadster/config/environment/enum.Environment.html) enum.

- `development` (alias: `dev`)
- `test`
- `production` (alias: `prod`)

In addition, apps can define a custom environment name, which is mapped to the `Environment::Custom` enum variant. This
is useful for special app-specific environments, such as additional pre-prod or canary environments.

## Docs.rs links

- [AppConfig](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html) struct