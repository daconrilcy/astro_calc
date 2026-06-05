use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};

use crate::error;
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

    if token.is_some_and(|t| constant_time_eq(t, expected)) {
        next.run(request).await
    } else {
        error::unauthorized()
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

fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/health/live" | "/health/ready" | "/v1/contracts" | "/openapi.yaml"
    ) || path.starts_with("/v1/schemas/")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_paths_are_exempt() {
        assert!(is_public_path("/health/live"));
        assert!(is_public_path("/v1/schemas/astro_engine_request_v1"));
        assert!(!is_public_path("/v1/calculations/natal"));
    }

    #[test]
    fn bearer_token_is_case_insensitive() {
        assert_eq!(
            bearer_token(Some(&"Bearer secret-key".parse().unwrap())),
            Some("secret-key")
        );
        assert_eq!(
            bearer_token(Some(&"bearer another".parse().unwrap())),
            Some("another")
        );
        assert_eq!(bearer_token(Some(&"Basic abc".parse().unwrap())), None);
    }
}
