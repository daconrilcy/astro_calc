use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DomainSelection {
    pub domain_count: u8,
    pub allowed_domains: Vec<String>,
    pub selected_domains: Option<Vec<String>>,
    pub selection_strategy: DomainSelectionStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DomainSelectionStrategy {
    Explicit,
    TopWeightedAstroSignals,
    ProductDefault,
}
