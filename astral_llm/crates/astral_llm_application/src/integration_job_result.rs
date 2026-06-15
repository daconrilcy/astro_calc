use astral_llm_domain::{integration::JobStatus, GenerateReadingResponse};
use serde_json::json;

pub fn unified_result_envelope(
    calculation: Option<serde_json::Value>,
    reading: &GenerateReadingResponse,
    reading_completeness: Option<String>,
) -> serde_json::Value {
    let mut out = json!({ "reading": reading });
    if let Some(token_usage) = reading.token_usage() {
        out["token_usage"] = serde_json::to_value(token_usage).unwrap_or(serde_json::Value::Null);
    }
    if let Some(calc) = calculation {
        out["calculation"] = calc;
    }
    if let Some(rc) = reading_completeness {
        out["reading_completeness"] = json!(rc);
    }
    out
}

pub fn job_error_from_reading(reading: &GenerateReadingResponse) -> serde_json::Value {
    match reading {
        GenerateReadingResponse::Failed {
            error: failure_error,
            ..
        } => {
            let mut error = json!({
                "code": failure_error.code.as_str(),
                "message": failure_error.message,
            });
            if let Some(details) = failure_error
                .details
                .as_ref()
                .filter(|details| !details.is_null())
            {
                error["details"] = details.clone();
            }
            error
        }
        GenerateReadingResponse::SafetyRejected {
            error: rejected_error,
            violations,
            ..
        } => {
            let mut error = json!({
                "code": "SAFETY_REJECTED",
                "message": rejected_error.message,
            });
            error["details"] = json!({
                "category": rejected_error.category,
                "rule_id": rejected_error.rule_id,
                "violations": violations,
            });
            error
        }
        _ => json!({ "code": "UNKNOWN", "message": "unexpected reading status" }),
    }
}

pub fn job_status_from_reading(reading: &GenerateReadingResponse) -> JobStatus {
    match reading {
        GenerateReadingResponse::Success { .. } => JobStatus::Completed,
        GenerateReadingResponse::SafetyRejected { .. } => JobStatus::SafetyRejected,
        GenerateReadingResponse::Failed { .. } => JobStatus::Failed,
    }
}
