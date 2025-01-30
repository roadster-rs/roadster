# Configuration

<!--Todo: Mention Environment::Custom variant-->
<!--Todo: Mention multiple file formats-->
<!--Todo: Mention overriding via env vars-->

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
- ./Cargo.toml
- ./config/default.toml
- ./config/default/db.toml
- ./config/development.toml
- ./config/development/db.toml
- ./config/test.toml
- ./config/test/db.toml
- ./config/prod.toml
- ./config/prod/db.toml
```

If there are multiple files in an environment's directory, they are loaded into the config in order, and the last
file loaded takes precedence if there are duplicate fields in the files.

## Environment variables

You can set environment variables to customize fields. This is useful to provide values for sensitive fields such as DB
passwords. Note that setting passwords as env vars does come with some amount of security risk as they are readable by
anyone who has access to your server, but they're better than checking your sensitive values into git or other source
control.

Env vars can either be set on the command line, or in a `.env` file. Environment variables should be prefixed with
`ROADSTER__` named according to the [AppConfig](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html)
structure, where each level of the structure is separated by a double underscore (`__`). For example, to set
`AppConfig#environment`, you would use an environment variable named `ROADSTER__ENVIRONMENT`. E.g.:

```shell
export ROADSTER__ENVIRONMENT=dev
```

## Config mechanism precedence

<!--Todo: Double check-->

- `default.toml` (lowest )
- `default/<filename>.toml`
- `<env>.toml`
- `<env>/<filename>.toml`
- Environment variables (highest -- overrides lower precedence values)

## Docs.rs links

- [AppConfig](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html) struct