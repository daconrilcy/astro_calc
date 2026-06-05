use serde_json::{json, Value};

use super::catalog::SimplifiedCatalog;
use super::facts::{CollectedSignFacts, RELIABILITY_DECLARED, RELIABILITY_STABLE};
use super::resolve::ResolvedSimplifiedInput;
use super::response::{
    AstroSimplifiedNatalResponse, InputPrecisionResponse, LimitationResponse, LlmPayloadControls,
    ReadingHintResponse, SIMPLIFIED_PAYLOAD_CONTRACT,
    SIMPLIFIED_RESPONSE_CONTRACT_VERSION, SimplifiedPayloadEnvelope,
};
use crate::domain::CalculatedChartFacts;

pub fn build_response(
    resolved: &ResolvedSimplifiedInput,
    catalog: &SimplifiedCatalog,
    collected: CollectedSignFacts,
    angular_facts: Option<&CalculatedChartFacts>,
) -> AstroSimplifiedNatalResponse {
    let limitations: Vec<LimitationResponse> = resolved
        .limitations
        .iter()
        .filter_map(|code| catalog.limitation(code))
        .map(|entry| LimitationResponse {
            code: entry.code.clone(),
            severity: entry.severity.clone(),
            affects: SimplifiedCatalog::affected_features(entry),
        })
        .collect();

    let llm_controls = build_llm_controls(resolved, catalog, &collected);

    AstroSimplifiedNatalResponse {
        response_contract_version: SIMPLIFIED_RESPONSE_CONTRACT_VERSION.to_string(),
        input_precision: InputPrecisionResponse {
            level: resolved.input_precision_level.clone(),
            date_provided: true,
            time_provided: resolved.birth_time.is_some(),
            timezone_provided: resolved.timezone.is_some(),
            location_provided: resolved.latitude.is_some(),
        },
        computed_scope: resolved.computed_scope.clone(),
        limitations,
        facts: collected.facts.clone(),
        ambiguous_facts: collected.ambiguous_facts.clone(),
        excluded_features: resolved.excluded_features.clone(),
        cusp_warnings: collected.cusp_warnings.clone(),
        simplified_payload: SimplifiedPayloadEnvelope {
            payload_contract: SIMPLIFIED_PAYLOAD_CONTRACT.to_string(),
            payload: build_simplified_payload(resolved, &collected, angular_facts),
        },
        llm_payload: llm_controls,
        reading_hint: ReadingHintResponse {
            recommended_profile_code: "natal_simplified".to_string(),
            reading_completeness: "partial".to_string(),
        },
    }
}

fn build_simplified_payload(
    resolved: &ResolvedSimplifiedInput,
    collected: &CollectedSignFacts,
    angular_facts: Option<&CalculatedChartFacts>,
) -> Value {
    let mut planets = json!({});
    if let Some(map) = planets.as_object_mut() {
        for fact in &collected.facts {
            if fact.reliability == RELIABILITY_STABLE || fact.reliability == RELIABILITY_DECLARED {
                map.insert(
                    fact.object_code.clone(),
                    json!({ "sign": fact.sign_code }),
                );
            }
        }
    }

    let mut payload = json!({
        "payload_contract": SIMPLIFIED_PAYLOAD_CONTRACT,
        "computed_scope": resolved.computed_scope,
        "input_precision_level": resolved.input_precision_level,
        "facts": collected.facts,
        "ambiguous_facts": collected.ambiguous_facts,
        "excluded_features": resolved.excluded_features,
        "planets": planets,
    });

    if let Some(facts) = angular_facts {
        payload["position_count"] = json!(facts.positions.len());
        payload["house_cusp_count"] = json!(facts.house_cusps.len());
        payload["aspect_count"] = json!(facts.aspects.len());
    }

    payload
}

fn build_llm_controls(
    resolved: &ResolvedSimplifiedInput,
    catalog: &SimplifiedCatalog,
    collected: &CollectedSignFacts,
) -> LlmPayloadControls {
    let allowed_fact_codes: Vec<String> = collected
        .facts
        .iter()
        .filter(|fact| catalog.allows_interpretive_affirmation(&fact.reliability))
        .map(|fact| format!("{}.sign", fact.object_code))
        .collect();

    let blocked_interpretation_fact_codes: Vec<String> = collected
        .ambiguous_facts
        .iter()
        .map(|fact| format!("{}.sign", fact.object_code))
        .collect();

    let excluded_feature_codes = resolved.excluded_features.clone();

    let mut allowed_limitation_mentions = blocked_interpretation_fact_codes.clone();
    for feature in &excluded_feature_codes {
        if !allowed_limitation_mentions.contains(feature) {
            allowed_limitation_mentions.push(feature.clone());
        }
    }

    let mut forbidden_topics = blocked_interpretation_fact_codes.clone();
    forbidden_topics.extend(excluded_feature_codes.clone());
    forbidden_topics.sort();
    forbidden_topics.dedup();

    LlmPayloadControls {
        profile_code: "natal_simplified".to_string(),
        allowed_fact_codes,
        blocked_interpretation_fact_codes,
        excluded_feature_codes,
        allowed_limitation_mentions,
        forbidden_topics: Some(forbidden_topics),
    }
}
