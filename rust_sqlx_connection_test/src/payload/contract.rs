use crate::domain::BasicLlmHandoffContract;
pub fn basic_llm_handoff_contract() -> BasicLlmHandoffContract {
    BasicLlmHandoffContract {
        contract_version: "natal_structured_v10".to_string(),
        payload_language_code: "en".to_string(),
        target_language_policy: "provided_by_llm_service".to_string(),
        audience_level: "beginner".to_string(),
        tone: "clear, warm, non fatalistic".to_string(),
        must_use: vec![
            "chart_context".to_string(),
            "chart_emphasis".to_string(),
            "rulership_context".to_string(),
            "dignities".to_string(),
            "angles".to_string(),
            "signals".to_string(),
            "reading_plan".to_string(),
            "drafting_plan".to_string(),
        ],
        must_not: vec![
            "invent facts not present in source signals".to_string(),
            "mention technical IDs".to_string(),
            "list placements mechanically".to_string(),
            "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group".to_string(),
            "expose raw evidence unless explicitly requested".to_string(),
            "treat chart_emphasis as a standalone section instead of weighting context".to_string(),
            "treat chart_context as a standalone section instead of contextual weighting".to_string(),
            "treat rulership_context as a standalone section instead of contextual weighting".to_string(),
            "make deterministic or fatalistic predictions".to_string(),
        ],
        output_format: "structured_sections".to_string(),
    }
}
