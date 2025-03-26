# CLI

When the `cli` feature is enabled, Roadster provides some CLI commands that are built into the app. Custom CLI commands
can also be added using [clap](https://docs.rs/clap/latest/clap/).

CLI commands run after the app is prepared (e.g., health checks, lifecycle handlers, and services registered, etc),
and before the health checks, lifecycle handlers, and services are run. This means that the app needs to have a valid
configuration in order to run a CLI command, but otherwise the app's resources (e.g. DB, Redis) don't need to be
healthy (unless, of course, the specific CLI command requires the resource).

## Adding custom CLI commands

```rust,ignore
{{#include ../../examples/cli/src/cli.rs:9:}}
```

## Sample CLI help text the above example

```text
$> ROADSTER__ENVIRONMENT=dev cargo run -- -h
A "Batteries Included" web framework for rust designed to get you moving fast.

CLI example: Commands specific to managing the `cli-example` app are provided in the CLI as well. Subcommands not listed under the `roadster` subcommand are specific to `cli-example`

Usage: cli-example [OPTIONS] [COMMAND]

Commands:
  roadster     Roadster subcommands. Subcommands provided by Roadster are listed under this subcommand in order to avoid naming conflicts with the consumer's subcommands [aliases: r]
  hello-world  Print a "hello world" message  
  help         Print this message or the help of the given subcommand(s)

Options:
  -e, --environment <ENVIRONMENT>      Specify the environment to use to run the application. This overrides the corresponding environment variable if it's set [possible values: development, test, production, <custom>]
      --skip-validate-config           Skip validation of the app config. This can be useful for debugging the app config when used in conjunction with the `print-config` command
      --allow-dangerous                Allow dangerous/destructive operations when running in the `production` environment. If this argument is not provided, dangerous/destructive operations will not be performed when running in `production`
      --config-dir <CONFIG_DIRECTORY>  The location of the config directory (where the app's config files are located). If not provided, will default to `./config/`
  -h, --help                           Print help
  -V, --version                        Print version
```

## Docs.rs links

- <https://docs.rs/roadster/latest/roadster/api/cli/trait.RunCommand.html>
- <https://docs.rs/roadster/latest/roadster/api/cli/roadster/index.html>
