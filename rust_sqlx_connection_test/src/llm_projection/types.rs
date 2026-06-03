use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmProjectionProfile {
    pub contract_version: String,
    pub level_code: String,
    pub max_keywords_per_item: usize,
    pub max_core_placements: usize,
    pub max_supporting_placements: usize,
    pub max_dominant_signs: usize,
    pub max_dominant_houses: usize,
    pub max_dominant_objects: usize,
    pub max_house_axes: usize,
    pub max_aspects: usize,
    pub include_accidental_conditions: bool,
    pub include_rulership_details: bool,
    pub include_minor_evidence: bool,
    pub include_degrees: bool,
    pub include_scores: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmProjectionNatalV1 {
    pub contract_version: String,
    pub projection_level: String,
    pub projection_limits: LlmProjectionLimitsEnvelope,
    pub chart: LlmChart,
    pub reading_order: Vec<LlmReadingOrderItem>,
    pub core_identity: LlmCoreIdentity,
    pub dominant_themes: LlmDominantThemes,
    pub placements: LlmPlacementsGroup,
    pub angles: LlmAngles,
    pub strengths: LlmStrengths,
    pub relationship_network: LlmRelationshipNetwork,
    pub dynamics: LlmDynamics,
    pub house_axes: Vec<LlmHouseAxis>,
    pub keywords: LlmKeywords,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmProjectionLimitsEnvelope {
    pub level: String,
    pub effective_limits: LlmEffectiveLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmEffectiveLimits {
    pub max_keywords_per_item: usize,
    pub max_core_placements: usize,
    pub max_supporting_placements: usize,
    pub max_dominant_signs: usize,
    pub max_dominant_houses: usize,
    pub max_dominant_objects: usize,
    pub max_house_axes: usize,
    pub max_aspects: usize,
    pub include_rulership_context: bool,
    pub include_accidental_dignities: bool,
    pub include_minor_evidence: bool,
    pub include_degrees: bool,
    pub include_scores: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmChart {
    #[serde(rename = "type")]
    pub chart_type: String,
    pub birth: LlmChartBirth,
    pub calculation: LlmChartCalculation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hemisphere_emphasis: Option<LlmHemisphereEmphasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmChartBirth {
    pub datetime_utc: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmChartCalculation {
    pub zodiac: String,
    pub coordinates: String,
    pub house_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmHemisphereEmphasis {
    pub dominant_area: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmReadingOrderItem {
    pub section: String,
    pub focus: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmCoreIdentity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sun: Option<LlmCoreBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moon: Option<LlmCoreBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ascendant: Option<LlmAscendantBody>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmCoreBody {
    pub placement: LlmPlacement,
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmAscendantBody {
    pub sign: String,
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ruler: Option<LlmAscendantRulers>,
    pub importance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmAscendantRulers {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traditional: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmDominantThemes {
    pub signs: Vec<LlmDominantSign>,
    pub houses: Vec<LlmDominantHouse>,
    pub objects: Vec<LlmDominantObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmDominantSign {
    pub sign: String,
    pub strength: String,
    pub reasons: Vec<String>,
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmDominantHouse {
    pub house: LlmHouseRef,
    pub strength: String,
    pub reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmDominantObject {
    pub object: String,
    pub strength: String,
    pub reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmPlacementsGroup {
    pub primary: Vec<LlmPlacement>,
    pub supporting: Vec<LlmPlacement>,
    pub background: Vec<LlmPlacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmPlacement {
    pub object: String,
    pub sign: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub house: Option<LlmHouseRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<String>,
    pub keywords: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub importance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude_deg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmHouseRef {
    pub number: i32,
    pub theme: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmAngles {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ascendant: Option<LlmAngleEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midheaven: Option<LlmAngleEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descendant: Option<LlmAngleEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imum_coeli: Option<LlmAngleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmAngleEntry {
    pub sign: String,
    pub house: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmStrengths {
    pub essential_dignities: Vec<LlmEssentialDignity>,
    pub accidental_conditions: Vec<LlmAccidentalCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmEssentialDignity {
    pub object: String,
    pub dignity: String,
    pub sign: String,
    pub effect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmAccidentalCondition {
    pub object: String,
    pub overall: String,
    pub conditions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overall_score: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmRelationshipNetwork {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ascendant_ruler: Option<LlmAscendantRulerNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midheaven_ruler: Option<LlmMcRulerNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dominant_dispositor: Option<LlmDominantDispositor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mutual_receptions: Vec<LlmMutualReception>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmAscendantRulerNetwork {
    pub ascendant_sign: String,
    pub main_ruler: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modern_ruler: Option<String>,
    pub ruler_placement: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmMcRulerNetwork {
    pub midheaven_sign: String,
    pub ruler: String,
    pub ruler_placement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmDominantDispositor {
    pub object: String,
    pub source_objects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmMutualReception {
    pub objects: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmDynamics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lunar_phase: Option<LlmLunarPhase>,
    #[serde(default)]
    pub major_aspects: Vec<LlmMajorAspect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmLunarPhase {
    pub phase: String,
    pub cycle: String,
    pub sun_moon_angle_degrees: f64,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmMajorAspect {
    pub aspect: String,
    pub quality: String,
    pub orb_degrees: f64,
    pub phase: String,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmHouseAxis {
    pub axis: String,
    pub houses: Vec<LlmHouseRef>,
    pub balance: String,
    pub importance: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmKeywords {
    pub main: Vec<String>,
    pub by_area: std::collections::BTreeMap<String, Vec<String>>,
}
