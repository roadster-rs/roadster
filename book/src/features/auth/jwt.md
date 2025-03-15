# JWTs

JSON Web Tokens (JWTs) are a common component of many auth systems. Each auth system will have a slightly different
set of fields available in the JWT; however, there are a few common fields as well as a few standards that could be
used by any implementation, particularly a custom auth system.

## JWT extractor

To make it easier to use JWTs with Roadster, we provide
an [Axum JWT extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.Jwt.html) to get the
JWT from the Bearer Authorization header. By default the extractor will extract all claims to the IETF standard (plus
any custom claims to a map), or a custom claim struct can be provided instead.

### IETF claims

```rust,ignore
{{#include ../../../examples/auth/src/jwt.rs:6:}}
```

### OpenID claims

Along with the IETF claims, Roadster also provides a claims struct to extract OpenID standard claims.

```rust,ignore
{{#include ../../../examples/auth/src/jwt_openid.rs:6:}}
```

## JwtCsrf extractor

Some apps may want or need to support clients that don't have javascript available. In those cases, the app will
typically
set an auth cookie so it can be sent automatically by the client on every request. However, _THIS MAY MAKE
THE APPLICATION VULNERABLE TO CSRF ATTACKS_.

Roadster provides a special [
`JwtCsrf`](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.JwtCsrf.html) extractor that allows
extracting a JWT either from the cookies sent by the client or the Bearer Authorization header as normal. The extractor
contains a special field that indicates whether it's safe to use or if the server needs to apply some CSRF protections
before the token can be safely used. The token is considered safe if it was extracted from the Bearer Authorization
header, or if the request is an HTTP verb that does not modify resources (e.g. `GET`). In any other case, e.g. the
JWT is extract from a cookie and the HTTP verb is a `POST`, the server must apply a CSRF protection mechanism before
using the token. If the `JwtCsrf` extractor is used for a route with a `GET` verb, take care not to modify resources,
otherwise the application will still be vulnerable to CSRF attacks.

See the following for more information and recommendations for how to implement CSRF protection mechanisms:

- <https://owasp.org/www-community/attacks/csrf>
- <https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html>

If the functionality to extract from a cookie is not required, it’s recommended to use the normal  [
`Jwt`](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.Jwt.html) extractor directly.

## Docs.rs links

- [`jwt` mod](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/index.html)
- [`Jwt` extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.Jwt.html)
- [`JwtCsrf` extractor](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/struct.JwtCsrf.html)
- [IETF claims](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/ietf/struct.Claims.html)
- [OpenID claims](https://docs.rs/roadster/latest/roadster/middleware/http/auth/jwt/openid/struct.Claims.html)
