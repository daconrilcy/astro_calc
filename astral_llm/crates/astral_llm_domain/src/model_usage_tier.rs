use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Contexte d'appel LLM pour valider le tier du modele (referentiel `llm_model_usage_tiers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelRouteContext {
    /// Lecture Premium/Basic, chapitres, single_pass.
    PrimaryReading,
    /// SummarySynthesizer, repair, validation, reformulation.
    Subtask,
    /// Benchmark oracle qualite (gpt-5.5-pro) — exige `allow_oracle_benchmark` sur la requete.
    OracleBenchmark,
}

/// Politique derivee du tier canonique en base (jointure `llm_model_usage_tiers`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct ModelUsageTierPolicy {
    pub allows_primary_reading: bool,
    pub allows_subtask: bool,
    pub allows_oracle_benchmark: bool,
}

impl ModelUsageTierPolicy {
    /// Moteurs sans tier explicite (fake, legacy) : autorises partout sauf oracle.
    pub fn unrestricted() -> Self {
        Self {
            allows_primary_reading: true,
            allows_subtask: true,
            allows_oracle_benchmark: false,
        }
    }

    pub fn allows(&self, context: ModelRouteContext) -> bool {
        match context {
            ModelRouteContext::PrimaryReading => self.allows_primary_reading,
            ModelRouteContext::Subtask => self.allows_subtask,
            ModelRouteContext::OracleBenchmark => self.allows_oracle_benchmark,
        }
    }
}
