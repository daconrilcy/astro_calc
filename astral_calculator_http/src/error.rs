//! Fabrique les reponses d'erreur HTTP normalisees pour l'API calculateur.
//! Le module centralise les codes, les messages et les conversions d'erreurs
//! internes afin de conserver un format stable cote client.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use tracing::error;
use uuid::Uuid;

/// Construit une reponse d'erreur JSON standardisee.
/// Tous les chemins d'erreur HTTP du service passent par cette fonction.
pub fn error_response(
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
    details: Option<Value>,
) -> Response {
    let body = json!({
        "status": "failed",
        "error": {
            "code": code,
            "message": message.into(),
            "details": details.unwrap_or_else(|| json!({}))
        },
        "request_id": Uuid::new_v4().to_string()
    });
    (status, Json(body)).into_response()
}

/// Produit une erreur 401 pour les requetes non authentifiees.
pub fn unauthorized() -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "UNAUTHORIZED",
        "Missing or invalid API key.",
        None,
    )
}

/// Produit une erreur 422 pour les payloads invalides ou incoherents.
pub fn validation_failed(message: impl Into<String>, details: Option<Value>) -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "VALIDATION_FAILED",
        message,
        details,
    )
}

/// Produit une erreur 503 lorsque le service n'est pas pret a traiter la requete.
pub fn service_not_ready(message: impl Into<String>, details: Value) -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "SERVICE_NOT_READY",
        message,
        Some(details),
    )
}

/// Produit une erreur 500 pour les defaillances internes non detaillees au client.
pub fn internal_error(message: impl Into<String>) -> Response {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "INTERNAL_ERROR",
        message,
        None,
    )
}

/// Produit une erreur 422 pour un echec de calcul metier.
pub fn calculation_failed(message: impl Into<String>, details: Option<Value>) -> Response {
    error_response(
        StatusCode::UNPROCESSABLE_ENTITY,
        "CALCULATION_FAILED",
        message,
        details,
    )
}

/// Convertit un rejet de JSON Axum en reponse d'erreur compatible API.
pub fn json_rejection(rejection: axum::extract::rejection::JsonRejection) -> Response {
    validation_failed(
        "Request body must be valid JSON.",
        Some(json!({ "errors": [rejection.to_string()] })),
    )
}

/// Transforme une erreur runtime du moteur en reponse HTTP adaptee.
pub fn map_runtime_error(err: astral_calculator::runtime::RuntimeError) -> Response {
    match err {
        astral_calculator::runtime::RuntimeError::InvalidEngineRequest(msg) => {
            validation_failed(msg, None)
        }
        astral_calculator::runtime::RuntimeError::Ephemeris(msg) => error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "EPHEMERIS_NOT_FOUND",
            msg,
            None,
        ),
        astral_calculator::runtime::RuntimeError::InvalidRuntimeTable(msg) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "REFERENCE_DATA_MISSING",
            msg,
            None,
        ),
        astral_calculator::runtime::RuntimeError::InvalidProjectionReasonDefinition(msg) => {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "REFERENCE_DATA_MISSING",
                msg,
                None,
            )
        }
        astral_calculator::runtime::RuntimeError::InvalidProjectionLabelDefinition(msg) => {
            error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "REFERENCE_DATA_MISSING",
                msg,
                None,
            )
        }
        astral_calculator::runtime::RuntimeError::Database(err) => {
            error!(error = %err, "database error");
            internal_error("An internal database error occurred.")
        }
        astral_calculator::runtime::RuntimeError::RunningCalculationInProgress {
            idempotency_key,
            chart_calculation_id,
        } => error_response(
            StatusCode::CONFLICT,
            "CALCULATION_IN_PROGRESS",
            "A calculation is already running for this idempotency key.",
            Some(json!({
                "idempotency_key": idempotency_key,
                "chart_calculation_id": chart_calculation_id
            })),
        ),
        astral_calculator::runtime::RuntimeError::Json(err) => validation_failed(
            "invalid JSON payload",
            Some(json!({ "errors": [err.to_string()] })),
        ),
    }
}
