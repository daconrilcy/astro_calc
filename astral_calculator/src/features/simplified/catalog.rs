//! Module astral_calculator\src\features\simplified\catalog.rs du moteur astral_calculator.

use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Clone)]
/// Structure SimplifiedCatalog.
pub struct SimplifiedCatalog {
    pub policy: SimplifiedPolicy,
    pub limitation_codes: Vec<LimitationCode>,
    pub reliability_levels: Vec<ReliabilityLevel>,
    pub calculation_scopes: Vec<CalculationScope>,
    pub input_precision_levels: Vec<InputPrecisionLevel>,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
/// Structure SimplifiedPolicy.
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
/// Structure LimitationCode.
pub struct LimitationCode {
    pub code: String,
    pub severity: String,
    pub affected_features_json: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
/// Structure ReliabilityLevel.
pub struct ReliabilityLevel {
    pub code: String,
    pub allows_interpretive_affirmation: bool,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
/// Structure CalculationScope.
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
/// Structure InputPrecisionLevel.
pub struct InputPrecisionLevel {
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, FromRow)]
/// Structure ProfileFeatureExclusion.
pub struct ProfileFeatureExclusion {
    pub profile_code: String,
    pub computed_scope_code: Option<String>,
    pub feature_code: String,
    pub exclusion_kind: String,
    pub sort_order: i32,
}

impl SimplifiedCatalog {
    /// Fonction limitation.
    pub fn limitation(&self, code: &str) -> Option<&LimitationCode> {
        self.limitation_codes
            .iter()
            .find(|entry| entry.code == code)
    }

    /// Fonction allows_interpretive_affirmation.
    pub fn allows_interpretive_affirmation(&self, reliability: &str) -> bool {
        self.reliability_levels
            .iter()
            .find(|level| level.code == reliability)
            .is_some_and(|level| level.allows_interpretive_affirmation)
    }

    /// Fonction scope.
    pub fn scope(&self, code: &str) -> Option<&CalculationScope> {
        self.calculation_scopes
            .iter()
            .find(|entry| entry.code == code)
    }

    /// Fonction input_precision.
    pub fn input_precision(&self, code: &str) -> Option<&InputPrecisionLevel> {
        self.input_precision_levels
            .iter()
            .find(|entry| entry.code == code)
    }

    /// Fonction affected_features.
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

    /// Exclusions profil actives : lignes globales (`computed_scope_code` null) + lignes scope-spécifiques.
    pub fn profile_feature_exclusions_for(
        exclusions: &[ProfileFeatureExclusion],
        profile_code: &str,
        computed_scope: &str,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for row in exclusions {
            if row.profile_code != profile_code {
                continue;
            }
            let scope_matches = row
                .computed_scope_code
                .as_ref()
                .map(|scope| scope == computed_scope)
                .unwrap_or(true);
            if scope_matches && !out.iter().any(|existing| existing == &row.feature_code) {
                out.push(row.feature_code.clone());
            }
        }
        out
    }
}
