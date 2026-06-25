//! Module astral_calculator\src\features\natal\signals\dignity.rs du moteur astral_calculator.

use std::collections::HashMap;

use serde_json::json;

use crate::domain::{CalculatedChartFacts, InterpretationSignalDraft, ObjectPositionFact};
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::dignities::{
    dignity_is_signal_worthy, dignity_source_weight_delta, essential_dignities_for_positions,
};

use super::constants::{SUPPRESSION_ACTIVE, THEME_FUNCTIONAL_CHALLENGE, THEME_FUNCTIONAL_STRENGTH};
use super::dignity_helpers::{
    dignity_evidence, dignity_interpretive_hint, dignity_priority, dignity_semantic_tags,
    dignity_summary, dignity_title,
};
use super::utils::round4;

pub(super) fn add_dignity_signals(
    facts: &CalculatedChartFacts,
    object_source_weights: &HashMap<&str, f64>,
    signals: &mut Vec<InterpretationSignalDraft>,
    catalog: &BasicPayloadCatalog,
    locale: &str,
) {
    let positions_by_object: HashMap<&str, &ObjectPositionFact> = facts
        .positions
        .iter()
        .map(|position| (position.object_code.as_str(), position))
        .collect();

    for dignity in essential_dignities_for_positions(&facts.positions, catalog)
        .into_iter()
        .filter(|dignity| dignity_is_signal_worthy(dignity, catalog))
    {
        let Some(position) = positions_by_object.get(dignity.object_code.as_str()) else {
            continue;
        };
        let theme_code = if dignity.polarity == "dignity" {
            THEME_FUNCTIONAL_STRENGTH
        } else {
            THEME_FUNCTIONAL_CHALLENGE
        };
        let title = dignity_title(&dignity, locale);
        let summary = dignity_summary(&dignity, locale);

        signals.push(InterpretationSignalDraft {
            signal_key: format!(
                "dignity:{}:{}:{}",
                dignity.object_code, dignity.dignity_type, dignity.sign_code
            ),
            signal_type_id: None,
            theme_code: Some(theme_code.to_string()),
            title,
            summary: Some(summary),
            priority_score: dignity_priority(&dignity, position, catalog),
            confidence_score: Some(0.95),
            suppression_state: SUPPRESSION_ACTIVE.to_string(),
            payload_json: Some(json!({
                "interpretive_hint": dignity_interpretive_hint(&dignity, locale),
                "semantic_tags": dignity_semantic_tags(&dignity),
                "source_weight": round4(
                    object_source_weights
                        .get(dignity.object_code.as_str())
                        .copied()
                        .unwrap_or(0.0)
                        + dignity_source_weight_delta(&dignity, catalog)
                ),
                "aggregation_group": format!("dignity:{}", dignity.object_code),
                "evidence": dignity_evidence(&dignity)
            })),
        });
    }
}
