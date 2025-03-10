# Auth

Auth (Authentication and Authorization) is an important component of virtually any app. There are many ways to go about
adding auth to an app, from implementing your own auth, to using a third-party OAuth service such as Auth0 or Clerk, or
using something like Supabase Auth that's somewhere in the middle.

Roadster's auth support is somewhat limited at the moment. However, a common component of many auth systems is Json Web
Tokens (JWTs). Roadster provides Axum extractors for a couple JWT standards that should help integrating with any
auth implementation you choose.

In the future, Roadster may provide a more opinionated solution for auth. For now, see the following chapters for
details about the auth features Roadster currently supports.
