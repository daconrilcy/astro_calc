use super::*;
#[derive(Debug, Clone, Serialize)]
pub(crate) struct PeriodV2QualityIssue {
    pub(crate) path: String,
    pub(crate) code: String,
    pub(crate) severity: String,
    pub(crate) message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopePublicRequest {
    pub date: String,
    pub timezone: String,
    pub target_language: String,
    pub chart_calculation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<HoroscopeLocation>,
    #[serde(default = "default_audience")]
    pub audience_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail_level: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopeLocation {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotProfile {
    pub service_code: String,
    pub slot_code: String,
    pub start_local_time: String,
    pub end_local_time: String,
    pub reference_local_time: String,
    pub slot_label: Option<String>,
    pub is_public: Option<bool>,
    pub sort_order: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoredSignal {
    pub evidence_key: String,
    pub fact_type: String,
    pub slot_id: String,
    pub source: String,
    pub transiting_object: String,
    pub natal_target: Option<String>,
    pub aspect: Option<String>,
    pub orb_deg: Option<f64>,
    pub natal_house: Option<i64>,
    pub theme_code: String,
    pub priority_score: f64,
    pub intensity: String,
    pub tone: String,
    pub duration_class: String,
    pub confidence_score: f64,
    pub human_label: String,
    pub score_breakdown: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlotInterpretationPlan {
    pub slot_code: String,
    pub slot_label: String,
    pub specificity: String,
    pub theme_code: Option<String>,
    pub tone: Option<String>,
    pub intensity: Option<String>,
    pub main_signal_keys: Vec<String>,
    pub required_evidence_keys: Vec<String>,
    pub advice_axis: Option<String>,
    pub avoid_axis: Option<String>,
    pub watch_point: Option<String>,
    pub best_for: Vec<String>,
    pub fallback_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TargetLanguageCode {
    Fr,
    En,
    Es,
    De,
}
impl TargetLanguageCode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            TargetLanguageCode::Fr => "fr",
            TargetLanguageCode::En => "en",
            TargetLanguageCode::Es => "es",
            TargetLanguageCode::De => "de",
        }
    }
    pub(crate) fn parse(value: &str) -> Result<Self, GenerationError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "fr" => Ok(TargetLanguageCode::Fr),
            "en" => Ok(TargetLanguageCode::En),
            "es" => Ok(TargetLanguageCode::Es),
            "de" => Ok(TargetLanguageCode::De),
            _ => Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                "HOROSCOPE_PERIOD_LANGUAGE_UNSUPPORTED",
                json!({ "target_language_code": value }),
            )),
        }
    }
}
pub(crate) fn default_target_language() -> String {
    "fr".to_string()
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AstrologerPersona {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persona_id: Option<String>,
    #[serde(default)]
    pub tone: Vec<String>,
    #[serde(default)]
    pub lexical_field: Vec<String>,
    #[serde(default)]
    pub priority_domains: Vec<String>,
    #[serde(default)]
    pub avoid_style: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpretation_style: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoroscopePeriodPublicRequest {
    pub anchor_date: String,
    pub timezone: String,
    #[serde(default = "default_target_language")]
    pub target_language: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_language_code: Option<TargetLanguageCode>,
    pub chart_calculation_id: String,
    #[serde(default = "default_audience")]
    pub audience_level: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub astrologer_persona: Option<AstrologerPersona>,
    #[serde(skip)]
    pub language_compat_warning: Option<Value>,
}
impl HoroscopePeriodPublicRequest {
    pub(crate) fn normalized_target_language_code(
        &self,
    ) -> Result<TargetLanguageCode, GenerationError> {
        self.target_language_code
            .clone()
            .map(Ok)
            .unwrap_or_else(|| TargetLanguageCode::parse(&self.target_language))
    }
}
pub(crate) fn default_audience() -> String {
    "general".into()
}
