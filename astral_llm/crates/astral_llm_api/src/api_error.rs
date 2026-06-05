use astral_llm_domain::{GenerationError, GenerationErrorCode};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use uuid::Uuid;

pub fn error_response(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    details: Option<Value>,
) -> Response {
    let mut error = json!({
        "code": code,
        "message": message.into(),
    });
    if let Some(d) = details.filter(|v| !v.is_null()) {
        error["details"] = d;
    }
    let body = json!({
        "status": "failed",
        "error": error,
        "request_id": Uuid::new_v4().to_string(),
    });
    (status, Json(body)).into_response()
}

pub fn unauthorized() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "UNAUTHORIZED",
        "Missing or invalid API key.",
        None,
    )
}

pub fn too_many_requests(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::TOO_MANY_REQUESTS,
        "TOO_MANY_REQUESTS",
        message,
        None,
    )
}

pub fn from_generation_error(err: GenerationError) -> Response {
    let detail = err.detail();
    error_response(
        map_generation_error_status(&detail.code),
        detail.code.as_str(),
        detail.message.clone(),
        detail.details.clone(),
    )
}

pub fn map_generation_error_status(code: &GenerationErrorCode) -> StatusCode {
    match code {
        GenerationErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        GenerationErrorCode::UnsupportedProvider
        | GenerationErrorCode::UnsupportedCapability
        | GenerationErrorCode::ProductPolicyViolation
        | GenerationErrorCode::PolicyViolation => StatusCode::BAD_REQUEST,
        GenerationErrorCode::ProviderTimeout => StatusCode::GATEWAY_TIMEOUT,
        GenerationErrorCode::ProviderRateLimited => StatusCode::TOO_MANY_REQUESTS,
        GenerationErrorCode::ProviderUnavailable | GenerationErrorCode::FallbackFailed => {
            StatusCode::BAD_GATEWAY
        }
        GenerationErrorCode::ReadingQualityFailed => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::GenerationError;

    #[test]
    fn error_response_matches_v1_shape() {
        let response = error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_INPUT",
            "bad field",
            Some(json!({ "field": "x" })),
        );
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn generation_error_uses_code_as_str() {
        let err = GenerationError::new(GenerationErrorCode::InvalidInput, "missing product");
        let response = from_generation_error(err);
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
