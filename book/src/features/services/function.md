# Function service

If you need to run some long-running service in your app and Roadster doesn't provide built-in support for the
specific service you need, you can implement [
`Service`](https://docs.rs/roadster/latest/roadster/service/trait.Service.html) directly. This gives you the
most control over the service, especially if you implement [
`ServiceBuilder`](https://docs.rs/roadster/latest/roadster/service/trait.ServiceBuilder.html) as well.

If you don't want to implement `Service` yourself, you can simply run the service in an `async` function and pass
that function to a [
`FunctionService`](https://docs.rs/roadster/latest/roadster/service/function/service/struct.FunctionService.html).

```rust,ignore
{{#include ../../../examples/service/src/function/mod.rs:8:}}
```

## Docs.rs links

- [`FunctionService`](https://docs.rs/roadster/latest/roadster/service/function/service/struct.FunctionService.html)
