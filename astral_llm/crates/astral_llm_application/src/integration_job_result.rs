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
        GenerateReadingResponse::SafetyRejected(r) => {
            let mut error = json!({
                "code": "SAFETY_REJECTED",
                "message": r.error.message,
            });
            error["details"] = json!({
                "category": r.error.category,
                "rule_id": r.error.rule_id,
                "violations": r.violations,
            });
            error
        }
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

#[cfg(test)]
mod tests {
    use super::job_error_from_reading;
    use astral_llm_domain::GenerationMode;
    use astral_llm_domain::{
        GenerateReadingResponse, GenerationErrorCode, LegalBlock, NatalReadingResponse,
        QualityMetadata, ReadingSummary, SafetyRejectedResponse, StructuredReadingResponse,
    };
    use serde_json::json;

    #[test]
    fn safety_rejected_job_error_keeps_category_rule_and_violations() {
        let reading = GenerateReadingResponse::SafetyRejected(SafetyRejectedResponse::new(
            "run-1",
            "safety_policy",
            "chapter failed safety validation",
            Some("SAFETY_SYMBOLIC_FRAMING".into()),
            vec!["missing symbolic/interpretive framing".into()],
        ));

        let error = job_error_from_reading(&reading);

        assert_eq!(error["code"], "SAFETY_REJECTED");
        assert_eq!(error["message"], "chapter failed safety validation");
        assert_eq!(error["details"]["category"], "safety_policy");
        assert_eq!(error["details"]["rule_id"], "SAFETY_SYMBOLIC_FRAMING");
        assert_eq!(
            error["details"]["violations"],
            json!(["missing symbolic/interpretive framing"])
        );
    }

    #[test]
    fn failed_job_error_still_keeps_original_details() {
        let reading =
            GenerateReadingResponse::Failed(astral_llm_domain::GenerationFailedResponse {
                run_id: "run-2".into(),
                error: astral_llm_domain::GenerationErrorDetail {
                    code: GenerationErrorCode::PostSafetyValidationFailed,
                    message: "failed".into(),
                    details: Some(json!({ "chapter": "identity" })),
                },
            });

        let error = job_error_from_reading(&reading);

        assert_eq!(error["code"], "POST_SAFETY_VALIDATION_FAILED");
        assert_eq!(error["details"]["chapter"], "identity");
    }

    #[test]
    fn success_job_error_falls_back_to_unknown() {
        let reading = GenerateReadingResponse::Success(StructuredReadingResponse {
            run_id: "run-3".into(),
            reading: NatalReadingResponse {
                schema_version: "natal_reading_v1".into(),
                language: "fr".into(),
                reading_type: "natal".into(),
                summary: ReadingSummary {
                    title: "t".into(),
                    short_text: "s".into(),
                },
                chapters: vec![],
                legal: LegalBlock {
                    disclaimer: "d".into(),
                },
                quality: QualityMetadata {
                    used_provider: "openai".into(),
                    used_model: "gpt-5-mini".into(),
                    generation_mode: GenerationMode::ChapterOrchestrated,
                    prompt_family: "natal_prompter".into(),
                    prompt_version: "v1".into(),
                    astro_contract_version: "natal_structured_v13".into(),
                    fallback_used: false,
                },
            },
        });

        let error = job_error_from_reading(&reading);

        assert_eq!(error["code"], "UNKNOWN");
    }
}
