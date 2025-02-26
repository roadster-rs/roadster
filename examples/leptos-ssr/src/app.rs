use leptos::prelude::*;
use leptos_meta::{MetaTags, provide_meta_context};

/// The static HTML shell of the app.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <link rel="stylesheet" id="leptos" href="/pkg/leptos-7-ssr-example.css" />

                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"Leptos 0.7 Example"</title>

                <AutoReload options=options.clone() />
                <HydrationScripts options islands=true />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Use the `view!` macro to build you UI using JSX-like syntax.
    view! {
        <HomePage/>
    }

    // Alternatively, you can use the builder syntax.
    // (
    //     HomePage(),
    // )
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
