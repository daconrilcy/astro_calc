//! Contrats du module de retraitement des textes LLM.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LANG_FR: &str = "fr";
pub const LANG_EN: &str = "en";
pub const LANG_ES: &str = "es";
pub const LANG_DE: &str = "de";

pub const SERVICE_HOROSCOPE_DAILY: &str = "horoscope_daily";
pub const SERVICE_HOROSCOPE_PERIOD: &str = "horoscope_period";
pub const SERVICE_NATAL_THEME: &str = "natal_theme";
pub const SERVICE_NATAL_SIMPLIFIED: &str = "natal_simplified";
pub const SERVICE_CALCULATOR_PROJECTION: &str = "calculator_projection";
pub const SERVICE_PROMPT_TRACE: &str = "prompt_trace";
pub const SERVICE_SHARED: &str = "shared";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TextLanguage {
    pub code: String,
}

impl TextLanguage {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TextService {
    pub code: String,
}

impl TextService {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextRetreatmentOperation {
    Sanitize,
    NormalizeDashes,
    Typography,
    HumanizeLabels,
    NormalizeLength,
    ReduceRepetition,
    ValidateQuality,
    BuildFallback,
    BuildPromptGuidance,
    FormatTrace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextTarget {
    PlainText,
    JsonPayload,
    NatalReading,
    HoroscopeDailyResponse,
    HoroscopePeriodResponse,
    PromptMessages,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TextWordLimits {
    pub min_words: Option<usize>,
    pub max_words: Option<usize>,
    pub hard_limit_words: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextRetreatmentRequestContext {
    pub profile_code: Option<String>,
    pub product_code: Option<String>,
    pub audience_level: Option<String>,
    pub word_limits: Option<TextWordLimits>,
    pub min_astro_basis_per_chapter: Option<usize>,
    pub allowed_evidence_keys: Vec<String>,
    pub allowed_evidence_by_chapter: Vec<TextChapterEvidenceKeys>,
    pub prior_chapters: Vec<String>,
    pub planet_display_names: Vec<String>,
    pub catalog_locale: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TextChapterEvidenceKeys {
    pub chapter_code: String,
    pub fact_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextRetreatmentRequest {
    pub language: TextLanguage,
    pub service: TextService,
    pub target: TextTarget,
    pub operations: Vec<TextRetreatmentOperation>,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub context: TextRetreatmentRequestContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextRetreatmentAuditAction {
    Changed,
    Validated,
    Skipped,
    FallbackApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TextRetreatmentAuditItem {
    pub processor_id: String,
    pub operation: TextRetreatmentOperation,
    pub field_path: Option<String>,
    pub action: TextRetreatmentAuditAction,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TextRetreatmentViolation {
    pub code: String,
    pub field_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextRetreatmentResponse {
    pub payload: serde_json::Value,
    pub audit: Vec<TextRetreatmentAuditItem>,
    pub warnings: Vec<String>,
    pub violations: Vec<TextRetreatmentViolation>,
    pub changed: bool,
}
