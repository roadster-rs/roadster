# JWTs

Json Web Tokens (JWTs) are a common component of many auth systems. Each auth system will have a slightly different
set of fields available in the JWT; however, there are a few common fields as well as a few standards that could be
used by any implementation, particularly a custom auth system.

## JWT extractor

To make it easier to use JWTs with Roadster, we provide
an [Axum JWT extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.Jwt.html) to get the
JWT from the Bearer Authorization header. By default the extractor will extract all claims to the IETF standard (plus
any custom claims to a map), or a custom claim struct can be provided instead.

### IETF

```rust,ignore
{{#include ../../../examples/auth/src/jwt.rs:7:}}
```

Along with the IETF claims, Roadster also provides a claims struct to extract OpenID standard claims.

### OpenID

```rust,ignore
{{#include ../../../examples/auth/src/jwt_openid.rs:6:}}
```

## JwtCsrf extractor

🛠️todo 🛠️

## Docs.rs links

- [`jwt` mod](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/index.html)
- [`Jwt` extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.Jwt.html)
- [`JwtCsrf` extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.JwtCsrf.html)
- [IETF claims](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/ietf/struct.Claims.html)
- [OpenID claims](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/openid/struct.Claims.html)
