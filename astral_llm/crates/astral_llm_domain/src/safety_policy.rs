use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct SafetyPolicy {
    pub forbid_medical_advice: bool,
    pub forbid_legal_advice: bool,
    pub forbid_financial_advice: bool,
    pub forbid_death_prediction: bool,
    pub forbid_pregnancy_prediction: bool,
    pub forbid_deterministic_claims: bool,
    pub require_symbolic_framing: bool,
    pub require_disclaimer: bool,
    #[serde(default)]
    pub custom_forbidden_topics: Vec<String>,
}

impl SafetyPolicy {
    pub fn mandatory() -> Self {
        Self {
            forbid_medical_advice: true,
            forbid_legal_advice: true,
            forbid_financial_advice: true,
            forbid_death_prediction: true,
            forbid_pregnancy_prediction: true,
            forbid_deterministic_claims: true,
            require_symbolic_framing: true,
            require_disclaimer: true,
            custom_forbidden_topics: Vec::new(),
        }
    }

    pub fn merge(base: &Self, overlay: &Self) -> Self {
        Self {
            forbid_medical_advice: base.forbid_medical_advice || overlay.forbid_medical_advice,
            forbid_legal_advice: base.forbid_legal_advice || overlay.forbid_legal_advice,
            forbid_financial_advice: base.forbid_financial_advice
                || overlay.forbid_financial_advice,
            forbid_death_prediction: base.forbid_death_prediction
                || overlay.forbid_death_prediction,
            forbid_pregnancy_prediction: base.forbid_pregnancy_prediction
                || overlay.forbid_pregnancy_prediction,
            forbid_deterministic_claims: base.forbid_deterministic_claims
                || overlay.forbid_deterministic_claims,
            require_symbolic_framing: base.require_symbolic_framing
                || overlay.require_symbolic_framing,
            require_disclaimer: base.require_disclaimer || overlay.require_disclaimer,
            custom_forbidden_topics: merge_topics(
                &base.custom_forbidden_topics,
                &overlay.custom_forbidden_topics,
            ),
        }
    }
}

fn merge_topics(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = a.to_vec();
    for topic in b {
        if !out.iter().any(|t| t.eq_ignore_ascii_case(topic)) {
            out.push(topic.clone());
        }
    }
    out
}

pub type SafetyPolicyOverride = SafetyPolicy;
