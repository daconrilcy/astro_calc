use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AstroFactKind {
    PlanetPosition,
    HousePlacement,
    Aspect,
    Dignity,
    DomainScore,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizedAstroFact {
    pub id: String,
    pub kind: AstroFactKind,
    pub label: String,
    pub value: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretive_weight: Option<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NormalizedAstroFacts {
    pub contract_version: String,
    pub facts: Vec<NormalizedAstroFact>,
}

impl NormalizedAstroFacts {
    pub fn fact_ids(&self) -> Vec<&str> {
        self.facts.iter().map(|f| f.id.as_str()).collect()
    }

    pub fn contains_fact(&self, fact_id: &str) -> bool {
        self.facts.iter().any(|f| f.id == fact_id)
    }
}
