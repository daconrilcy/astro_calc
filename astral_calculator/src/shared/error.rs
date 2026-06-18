//! Module astral_calculator\src\shared\error.rs du moteur astral_calculator.

use thiserror::Error;

#[derive(Debug, Error)]
/// Enum RuntimeError.
pub enum RuntimeError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ephemeris error: {0}")]
    Ephemeris(String),
    #[error("invalid runtime table: {0}")]
    InvalidRuntimeTable(String),
    #[error("calculation is already running for idempotency key {idempotency_key}")]
    RunningCalculationInProgress {
        idempotency_key: String,
        chart_calculation_id: i32,
    },
    #[error("invalid engine request: {0}")]
    InvalidEngineRequest(String),
    #[error("invalid projection reason definition: {0}")]
    InvalidProjectionReasonDefinition(String),
    #[error("invalid projection label definition: {0}")]
    InvalidProjectionLabelDefinition(String),
}

impl RuntimeError {
    /// Fonction code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Database(_) => "database_error",
            Self::Json(_) => "json_error",
            Self::Ephemeris(_) => "ephemeris_error",
            Self::InvalidRuntimeTable(_) => "invalid_runtime_table",
            Self::RunningCalculationInProgress { .. } => "running_calculation_in_progress",
            Self::InvalidEngineRequest(_) => "invalid_engine_request",
            Self::InvalidProjectionReasonDefinition(_) => "invalid_projection_reason_definition",
            Self::InvalidProjectionLabelDefinition(_) => "invalid_projection_label_definition",
        }
    }
}
