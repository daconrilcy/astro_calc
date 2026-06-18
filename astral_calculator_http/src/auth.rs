//! Middleware d'authentification applique aux routes protegees du service HTTP.
//! Le module autorise certaines routes publiques, puis verifie une cle API via
//! `Authorization: Bearer ...` ou `x-api-key` selon la configuration.

use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};

use crate::error;
use crate::state::AppState;

/// Verifie si la requete peut continuer vers le routeur protege.
/// Les routes publiques passent sans controle; sinon la cle fournie est comparee
/// en temps quasi constant pour limiter les risques de fuite par temporisation.
pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !state.config.requires_auth() || is_public_path(request.uri().path()) {
        return next.run(request).await;
    }

    let expected = state.config.api_key.as_deref().unwrap_or("");
    let token = bearer_token(request.headers().get("authorization")).or_else(|| {
        request
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
    });

    if token.is_some_and(|t| constant_time_eq(t, expected)) {
        next.run(request).await
    } else {
        error::unauthorized()
    }
}

/// Extrait un token Bearer depuis un en-tete HTTP `Authorization`.
/// Retourne `None` si le format est invalide ou si le jeton est vide.
fn bearer_token(value: Option<&axum::http::HeaderValue>) -> Option<&str> {
    let raw = value?.to_str().ok()?;
    let scheme_end = raw.find(' ')?;
    if !raw[..scheme_end].eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = raw[scheme_end + 1..].trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Indique si le chemin fait partie des routes exposes sans authentification.
fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/health/live" | "/health/ready" | "/v1/contracts" | "/openapi.yaml"
    ) || path.starts_with("/v1/schemas/")
}

/// Compare deux chaines en limitant les variations de temps d'execution.
/// Cette comparaison n'est utilisee que pour des secrets de meme longueur.
fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
