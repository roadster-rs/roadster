# Configuration

<!--Todo: Mention Environment::Custom variant-->
<!--Todo: Mention multiple file formats-->
<!--Todo: Mention overriding via env vars-->

Roadster provides sensible defaults, but is highly customizable via configuration files and environment variables.

## Configuration files

🛠️ todo 🛠️

## Environment variables

You can set environment variables to customize fields. These can either be set on the command line, or in a `.env` file.
Environment variables should be prefixed with `ROADSTER__` named according to
the [AppConfig](https://docs.rs/roadster/latest/roadster/config/struct.AppConfig.html) structure, where each level of
the structure is separated by a double underscore (`__`). For example, to set `AppConfig#environment`, you would use an
environment variable named `ROADSTER__ENVIRONMENT`. E.g.:

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