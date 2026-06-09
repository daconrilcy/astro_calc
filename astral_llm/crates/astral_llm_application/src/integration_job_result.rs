use astral_llm_domain::{integration::JobStatus, GenerateReadingResponse};
use serde_json::json;

pub fn unified_result_envelope(
    calculation: Option<serde_json::Value>,
    reading: &GenerateReadingResponse,
    reading_completeness: Option<String>,
) -> serde_json::Value {
    let mut out = json!({ "reading": reading });
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
        GenerateReadingResponse::Failed(f) => {
            let mut error = json!({
                "code": f.error.code.as_str(),
                "message": f.error.message,
            });
            if let Some(details) = f
                .error
                .details
                .as_ref()
                .filter(|details| !details.is_null())
            {
                error["details"] = details.clone();
            }
            error
        }
        GenerateReadingResponse::SafetyRejected(r) => json!({
            "code": "SAFETY_REJECTED",
            "message": r.error.message,
        }),
        _ => json!({ "code": "UNKNOWN", "message": "unexpected reading status" }),
    }
}

pub fn job_status_from_reading(reading: &GenerateReadingResponse) -> JobStatus {
    match reading {
        GenerateReadingResponse::Success(_) => JobStatus::Completed,
        GenerateReadingResponse::SafetyRejected(_) => JobStatus::SafetyRejected,
        GenerateReadingResponse::Failed(_) => JobStatus::Failed,
    }
}
