use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Clone)]
pub struct SimplifiedCatalog {
    pub policy: SimplifiedPolicy,
    pub limitation_codes: Vec<LimitationCode>,
    pub reliability_levels: Vec<ReliabilityLevel>,
    pub calculation_scopes: Vec<CalculationScope>,
    pub input_precision_levels: Vec<InputPrecisionLevel>,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct SimplifiedPolicy {
    pub code: String,
    pub reference_time_utc: String,
    pub date_only_uncertainty_mode: String,
    pub uncertainty_sampling_minutes: i32,
    pub default_timezone_strategy: String,
    pub cusp_warning_orb_deg: f64,
    pub stable_fact_strategy: String,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct LimitationCode {
    pub code: String,
    pub severity: String,
    pub affected_features_json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct ReliabilityLevel {
    pub code: String,
    pub allows_interpretive_affirmation: bool,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct CalculationScope {
    pub code: String,
    pub min_input_precision_code: String,
    pub supports_angles: bool,
    pub supports_houses: bool,
    pub supports_aspects: bool,
    pub supports_object_sign_facts: bool,
    pub supports_ambiguous_facts: bool,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
pub struct InputPrecisionLevel {
    pub code: String,
}

impl SimplifiedCatalog {
    pub fn limitation(&self, code: &str) -> Option<&LimitationCode> {
        self.limitation_codes.iter().find(|entry| entry.code == code)
    }

    pub fn allows_interpretive_affirmation(&self, reliability: &str) -> bool {
        self.reliability_levels
            .iter()
            .find(|level| level.code == reliability)
            .is_some_and(|level| level.allows_interpretive_affirmation)
    }

    pub fn scope(&self, code: &str) -> Option<&CalculationScope> {
        self.calculation_scopes.iter().find(|entry| entry.code == code)
    }

    pub fn input_precision(&self, code: &str) -> Option<&InputPrecisionLevel> {
        self.input_precision_levels
            .iter()
            .find(|entry| entry.code == code)
    }

    pub fn affected_features(limitation: &LimitationCode) -> Vec<String> {
        limitation
            .affected_features_json
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }
}
