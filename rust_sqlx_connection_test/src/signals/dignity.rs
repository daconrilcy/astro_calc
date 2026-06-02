use std::collections::HashMap;

use serde_json::json;

use crate::dignities::{
    dignity_is_signal_worthy, dignity_source_weight_delta, essential_dignities_for_positions,
};
use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft};

use super::constants::{SUPPRESSION_ACTIVE, THEME_FUNCTIONAL_CHALLENGE, THEME_FUNCTIONAL_STRENGTH};
use super::dignity_helpers::{
    dignity_evidence, dignity_interpretive_hint, dignity_priority, dignity_semantic_tags,
    dignity_summary, dignity_title, dignity_writing_guidance,
};
use super::utils::round4;

pub(super) fn add_dignity_signals(
    facts: &CalculatedChartFacts,
    object_source_weights: &HashMap<&str, f64>,
    signals: &mut Vec<InterpretationSignalDraft>,
) {
    for dignity in essential_dignities_for_positions(&facts.positions)
        .into_iter()
        .filter(dignity_is_signal_worthy)
    {
        let theme_code = if dignity.polarity == "dignity" {
            THEME_FUNCTIONAL_STRENGTH
        } else {
            THEME_FUNCTIONAL_CHALLENGE
        };
        let title = dignity_title(&dignity);
        let summary = dignity_summary(&dignity);

        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "dignity:{}:{}:{}",
                dignity.object_code, dignity.dignity_type, dignity.sign_code
            ),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
            title,
            summary: Some(summary),
            priority_score: dignity_priority(&dignity),
            confidence_score: Some(0.95),
            suppression_state: SUPPRESSION_ACTIVE.to_string(),
            payload_json: Some(json!({
                "interpretive_hint": dignity_interpretive_hint(&dignity),
                "semantic_tags": dignity_semantic_tags(&dignity),
                "source_weight": round4(
                    object_source_weights
                        .get(dignity.object_code.as_str())
                        .copied()
                        .unwrap_or(0.0)
                        + dignity_source_weight_delta(&dignity)
                ),
                "aggregation_group": format!("dignity:{}", dignity.object_code),
                "writing_guidance": dignity_writing_guidance(&dignity),
                "evidence": dignity_evidence(&dignity)
            })),
        });
    }
}
