# gRPC service with [Tonic](https://docs.rs/tonic/latest/tonic/)

Roadster best supports API services using Axum. However, we do provide a gRPC service via a basic integration
with [`tonic`](https://docs.rs/tonic/latest/tonic/). Support is pretty minimal and you'll need to manage building
your gRPC Router yourself. However, once it's built, Roadster can take care of running it for you.

```rust,ignore
{{#include ../../../examples/service/src/grpc/mod.rs:12:}}
```

## Docs.rs links

- [`tonic`](https://docs.rs/tonic/latest/tonic/)
- [`GrpcService`](https://docs.rs/roadster/latest/roadster/service/grpc/service/struct.GrpcService.html)
