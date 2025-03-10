use axum::response::IntoResponse;
use roadster::middleware::http::auth::jwt::{Jwt, openid};
use tracing::info;
use uuid::Uuid;

/// Example extracting the OpenID [`Claims`] and a map of custom fields for the JWT.
async fn api_with_openid_claims(jwt: Jwt<openid::Claims>) -> impl IntoResponse {
    info!(subject=?jwt.claims.subject, user=?jwt.claims.custom.get("userId"), "Handling request");
}

struct CustomClaims {
    user_id: Uuid,
}

/// Example extracting both the OpenID [`Claims`] and the [`CustomClaims`] for the JWT.
async fn api_with_openid_and_custom_claims(
    jwt: Jwt<openid::Claims<CustomClaims>>,
) -> impl IntoResponse {
    info!(subject=?jwt.claims.subject, user=%jwt.claims.custom.user_id,
        "Handling request",
    );
}
