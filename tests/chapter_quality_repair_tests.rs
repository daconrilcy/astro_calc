use astral_llm_application::chapter_quality_repair::safety_repair_from_error;
use astral_llm_domain::{GenerationError, GenerationErrorCode};

#[test]
fn safety_repair_detects_symbolic_framing_violation() {
    let err = GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        "chapter failed safety validation",
        serde_json::json!({ "violations": ["missing symbolic/interpretive framing"] }),
    );
    assert_eq!(
        safety_repair_from_error(&err),
        Some(astral_llm_application::chapter_quality_repair::ChapterRepairKind::SymbolicFraming)
    );
}

#[test]
fn safety_repair_ignores_other_violations() {
    let err = GenerationError::with_details(
        GenerationErrorCode::PostSafetyValidationFailed,
        "chapter failed safety validation",
        serde_json::json!({ "violations": ["medical advice detected"] }),
    );
    assert_eq!(safety_repair_from_error(&err), None);
}
