# Leptos SSR Roadster example

This is an example for how to use [Leptos](https://github.com/leptos-rs/leptos) in SSR mode with Roadster.

```bash
cd leptos-ssr-example
```

to go to your newly created project.  
Feel free to explore the project structure, but the best place to start with your application code is in `src/app.rs`.  
Addtionally, Cargo.toml may need updating as new versions of the dependencies are released, especially if things are not
working after a `cargo update`.

## Running your project

```bash
# From the root Roadster directory, cd into the example dir
cd examples/leptos-ssr
# Run the app
ROADSTER__ENVIRONMENT=development cargo leptos watch -- --config-dir "$(pwd)/config"
# Alternatively, you can put the ROADSTER__ENVIRONMENT=development in a `.env` file and simply run
cargo leptos watch -- --config-dir "$(pwd)/config"
```
