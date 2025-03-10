use axum::response::IntoResponse;
use roadster::middleware::http::auth::jwt::{Jwt, ietf};
use tracing::info;
use uuid::Uuid;

/// Example extracting the IETF [`ietf::Claims`] and a map of custom fields for the JWT.
async fn api_with_ietf_claims(jwt: Jwt) -> impl IntoResponse {
    info!(subject=?jwt.claims.subject, user=?jwt.claims.custom.get("userId"), "Handling request");
}

struct CustomClaims {
    user_id: Uuid,
}

/// Example extracting both the IETF [`ietf::Claims`] and the [`CustomClaims`] for the JWT.
async fn api_with_ietf_and_custom_claims(
    jwt: Jwt<ietf::Claims<CustomClaims>>,
) -> impl IntoResponse {
    info!(subject=?jwt.claims.subject, user=%jwt.claims.custom.user_id,
        "Handling request",
    );
}

/// Example using the [`CustomClaims`] as the only claims extracted for the JWT.
async fn api_with_custom_claims(jwt: Jwt<CustomClaims>) -> impl IntoResponse {
    info!(user=%jwt.claims.user_id, "Handling request",);
}
