use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};

use crate::api_error::unauthorized;
use crate::state::AppState;

pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if !state.config.requires_auth() || is_public_path(request.uri().path()) {
        return next.run(request).await;
    }

    let expected = state.config.api_key.as_deref().unwrap_or("");
    let token = bearer_token(request.headers().get("authorization"))
        .or_else(|| {
            request
                .headers()
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
        });

    let authorized = token.is_some_and(|t| constant_time_eq(t, expected));

    if authorized {
        next.run(request).await
    } else {
        unauthorized()
    }
}

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

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/health/live" | "/health/ready" | "/v1/contracts" | "/openapi.yaml"
    ) || path.starts_with("/v1/schemas/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_checks() {
        assert!(constant_time_eq("secret", "secret"));
        assert!(!constant_time_eq("secret", "Secret"));
        assert!(!constant_time_eq("secret", "secrets"));
    }
}
