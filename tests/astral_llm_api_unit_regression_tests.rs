use astral_llm_api::api_error::{error_response, from_generation_error};
use astral_llm_domain::{GenerationError, GenerationErrorCode};
use axum::{body::to_bytes, http::StatusCode};
use serde_json::Value;

#[tokio::test]
async fn llm_api_error_response_keeps_v1_envelope_shape() {
    let response = error_response(
        StatusCode::BAD_REQUEST,
        "INVALID_INPUT",
        "bad field",
        Some(serde_json::json!({ "field": "x" })),
    );
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: Value = serde_json::from_slice(
        &to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body"),
    )
    .expect("json");
    assert_eq!(body["status"], "failed");
    assert_eq!(body["error"]["code"], "INVALID_INPUT");
    assert_eq!(body["error"]["details"]["field"], "x");
    assert!(body["request_id"].as_str().is_some());
}

#[test]
fn llm_generation_error_maps_invalid_input_to_bad_request() {
    let err = GenerationError::new(GenerationErrorCode::InvalidInput, "missing product");
    let response = from_generation_error(err);
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
