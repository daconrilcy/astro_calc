use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AstrologerProfile {
    pub profile_id: Option<String>,
    pub name: Option<String>,
    pub tone: ToneProfile,
    pub jargon_level: JargonLevel,
    pub wording_style: WordingStyle,
    pub preferred_domains: Vec<String>,
    pub forbidden_wording: Vec<String>,
    pub custom_instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JargonLevel {
    Beginner,
    Balanced,
    Expert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToneProfile {
    Warm,
    Direct,
    Spiritual,
    Psychological,
    Traditional,
    Modern,
    Poetic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WordingStyle {
    Clear,
    Narrative,
    Analytical,
    Poetic,
}
