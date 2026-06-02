use crate::domain::BasicPayload;

pub(super) fn has_current_llm_handoff_contract(payload: &BasicPayload) -> bool {
    let Some(contract) = payload.llm_handoff_contract.as_ref() else {
        return false;
    };

    contract.contract_version == "basic_natal_structured_v8"
        && contract.payload_language_code == "en"
        && contract.target_language_policy == "provided_by_llm_service"
        && contract.audience_level == "beginner"
        && contract.tone == "clear, warm, non fatalistic"
        && contract.must_use.as_slice()
            == [
                "chart_context",
                "chart_emphasis",
                "dignities",
                "angles",
                "signals",
                "reading_plan",
                "drafting_plan",
            ]
        && contract.must_not.as_slice()
            == [
                "invent facts not present in source signals",
                "mention technical IDs",
                "list placements mechanically",
                "translate technical keys such as signal_key, theme_code, semantic_tags, slot, or aggregation_group",
                "expose raw evidence unless explicitly requested",
                "treat chart_emphasis as a standalone section instead of weighting context",
                "make deterministic or fatalistic predictions",
            ]
        && contract.output_format == "structured_sections"
}
