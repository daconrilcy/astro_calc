use astral_contracts::ErrorResponseCommon;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Upstream(String),
    #[error("{0}")]
    Internal(String),
}

impl GatewayError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }

    pub fn upstream(message: impl Into<String>) -> Self {
        Self::Upstream(message.into())
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "INVALID_INPUT"),
            Self::Upstream(_) => (StatusCode::BAD_GATEWAY, "UPSTREAM_FAILURE"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        let error = ErrorResponseCommon {
            code: code.to_string(),
            message: self.to_string(),
            details: None,
        };
        (
            status,
            Json(json!({
                "status": "failed",
                "error": error,
            })),
        )
            .into_response()
    }
}
