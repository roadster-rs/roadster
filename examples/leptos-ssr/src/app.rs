use leptos::*;
use leptos_meta::*;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Use the `view!` macro to build you UI using JSX-like syntax.
    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/leptos-ssr-example.css" />

        // sets the document title
        <Title text="Welcome to Leptos" />

        <HomePage/>
    }

    // Alternatively, you can use the builder syntax.
    // (
    //     StylesheetProps::builder()
    //         .id("leptos")
    //         .href("/pkg/leptos-ssr-example.css")
    //         .build(),
    //     TitleProps::builder().text("Welcome to Leptos").build(),
    //     HomePage(),
    // )
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let (count, set_count) = create_signal(0);
    let on_click = move |_| set_count.update(|count| *count += 1);

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>"Click Me: " {count}</button>
    }
}
