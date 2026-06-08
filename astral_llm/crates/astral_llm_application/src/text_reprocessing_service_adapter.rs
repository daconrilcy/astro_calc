//! Adaptateurs applicatifs vers le pipeline `text_reprocessing`.

use astral_llm_domain::{
    generation_response::NatalReadingResponse, TextLanguage, TextRetreatmentAuditAction,
    TextRetreatmentOperation as Op, TextRetreatmentRequest, TextRetreatmentRequestContext,
    TextRetreatmentResponse, TextService, TextTarget, TextWordLimits,
    SERVICE_CALCULATOR_PROJECTION, SERVICE_HOROSCOPE_DAILY, SERVICE_HOROSCOPE_PERIOD,
    SERVICE_NATAL_SIMPLIFIED, SERVICE_NATAL_THEME, SERVICE_PROMPT_TRACE, SERVICE_SHARED,
};
use astral_llm_providers::{PromptMessage, PromptRole};
use serde_json::{json, Value};

use crate::text_reprocessing::TextRetreatmentPipeline;

#[derive(Debug, Clone)]
pub struct TextReprocessingApplicationError {
    pub message: String,
}

impl std::fmt::Display for TextReprocessingApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TextReprocessingApplicationError {}

impl From<serde_json::Error> for TextReprocessingApplicationError {
    fn from(err: serde_json::Error) -> Self {
        Self {
            message: err.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextReprocessingFieldAudit {
    pub sanitized_fields: Vec<String>,
    pub typography_fields: Vec<String>,
    pub fallback_fields: Vec<String>,
    pub validated_fields: Vec<String>,
}

pub fn reprocess_shared_text(language: &str, text: &str) -> TextRetreatmentResponse {
    run(
        language,
        SERVICE_SHARED,
        TextTarget::PlainText,
        vec![Op::Sanitize, Op::Typography],
        json!(text),
        TextRetreatmentRequestContext::default(),
    )
}

pub fn reprocess_prompt_trace(messages: &[PromptMessage]) -> String {
    let payload = json!({
        "messages": messages.iter().map(prompt_message_to_json).collect::<Vec<_>>()
    });
    let response = run(
        "en",
        SERVICE_PROMPT_TRACE,
        TextTarget::PromptMessages,
        vec![Op::FormatTrace],
        payload,
        TextRetreatmentRequestContext::default(),
    );
    response
        .payload
        .get("formatted_trace")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn reprocess_calculator_projection(language: &str, payload: Value) -> TextRetreatmentResponse {
    run(
        language,
        SERVICE_CALCULATOR_PROJECTION,
        TextTarget::JsonPayload,
        vec![Op::HumanizeLabels, Op::Sanitize],
        payload,
        TextRetreatmentRequestContext::default(),
    )
}

pub fn reprocess_natal_simplified(
    reading: &mut NatalReadingResponse,
    language: &str,
    operations: Vec<Op>,
) -> Result<TextReprocessingFieldAudit, TextReprocessingApplicationError> {
    let original = serde_json::to_value(&*reading)?;
    let response = run(
        language,
        SERVICE_NATAL_SIMPLIFIED,
        TextTarget::NatalReading,
        operations,
        original,
        TextRetreatmentRequestContext::default(),
    );
    let audit = field_audit_from_response(&response);
    *reading = serde_json::from_value(response.payload)?;
    remove_chapter_symbolic_disclaimer_boilerplate(reading);
    Ok(audit)
}

fn remove_chapter_symbolic_disclaimer_boilerplate(reading: &mut NatalReadingResponse) {
    for chapter in &mut reading.chapters {
        chapter.body = chapter
            .body
            .split("\n\n")
            .filter_map(|paragraph| {
                let cleaned = remove_symbolic_boilerplate_sentences(paragraph);
                let cleaned = cleaned.trim();
                (!cleaned.is_empty()).then(|| cleaned.to_string())
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
}

fn remove_symbolic_boilerplate_sentences(paragraph: &str) -> String {
    let mut kept = Vec::new();
    let mut start = 0;
    for (index, ch) in paragraph.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let end = index + ch.len_utf8();
            let sentence = paragraph[start..end].trim();
            let cleaned = strip_symbolic_boilerplate_fragments(sentence);
            if !cleaned.is_empty() {
                kept.push(cleaned);
            }
            start = end;
        }
    }
    let tail = paragraph[start..].trim();
    if !tail.is_empty() {
        let cleaned = strip_symbolic_boilerplate_fragments(tail);
        if !cleaned.is_empty() {
            kept.push(cleaned);
        }
    }
    kept.join(" ")
}

fn strip_symbolic_boilerplate_fragments(text: &str) -> String {
    let mut cleaned = text.to_string();
    for phrase in [
        "Cette lecture reste symbolique et exploratoire, non deterministe.",
        "Cette lecture reste symbolique et exploratoire, non déterministe.",
        "Cette lecture reste symbolique et exploratoire.",
        "Cette lecture reste symbolique.",
        "Lecture symbolique et non deterministe, bien sur ; elle decrit une tendance de fond, pas une destinee fermee.",
        "Lecture symbolique et non déterministe, bien sûr ; elle décrit une tendance de fond, pas une destinée fermée.",
        "Comme toujours en astrologie, il s’agit d’une lecture exploratoire, non déterministe, de vos tendances profondes.",
        "Comme toujours en astrologie, il s'agit d'une lecture exploratoire, non deterministe, de vos tendances profondes.",
        "lecture astrologique reste symbolique",
        "Lecture astrologique reste symbolique",
        "cette lecture reste symbolique",
        "Cette lecture reste symbolique",
        "lecture reste symbolique",
        "Lecture reste symbolique",
        "dans une lecture symbolique",
        "Dans une lecture symbolique",
    ] {
        cleaned = cleaned.replace(phrase, "");
    }
    let cleaned = cleaned.trim().trim_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '.' | '!' | '?' | ',' | ';' | ':')
    });
    if is_symbolic_boilerplate_sentence(cleaned) {
        return String::new();
    }
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn is_symbolic_boilerplate_sentence(sentence: &str) -> bool {
    let lower = sentence.to_lowercase();
    let normalized = lower.trim_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '.' | '!' | '?' | ',' | ';' | ':')
    });
    let is_reading_disclaimer = (normalized.starts_with("cette lecture")
        || normalized.starts_with("lecture symbolique")
        || normalized.starts_with("la lecture")
        || normalized.starts_with("comme toujours en astrologie"))
        && (normalized.contains("reste symbolique")
            || normalized.contains("exploratoire")
            || normalized.contains("non déterministe")
            || normalized.contains("non deterministe")
            || normalized.contains("rien de certain"));
    normalized.is_empty()
        || normalized == "cette lecture reste"
        || normalized == "cette lecture"
        || normalized == "lecture astrologique reste"
        || normalized == "lecture reste"
        || normalized == "comme toujours en astrologie"
        || normalized
            == "il s’agit d’une lecture exploratoire, non déterministe, de vos tendances profondes"
        || normalized
            == "il s'agit d'une lecture exploratoire, non deterministe, de vos tendances profondes"
        || normalized == "thème n’annonce rien de certain"
        || normalized == "theme n'annonce rien de certain"
        || is_reading_disclaimer
}

pub fn reprocess_horoscope_daily(
    language: &str,
    payload: Value,
    word_limits: Option<TextWordLimits>,
) -> TextRetreatmentResponse {
    run(
        language,
        SERVICE_HOROSCOPE_DAILY,
        TextTarget::HoroscopeDailyResponse,
        vec![
            Op::Sanitize,
            Op::Typography,
            Op::NormalizeLength,
            Op::ReduceRepetition,
            Op::ValidateQuality,
            Op::BuildFallback,
        ],
        payload,
        context_with_word_limits(word_limits),
    )
}

pub fn reprocess_horoscope_period(
    language: &str,
    payload: Value,
    word_limits: Option<TextWordLimits>,
) -> TextRetreatmentResponse {
    run(
        language,
        SERVICE_HOROSCOPE_PERIOD,
        TextTarget::HoroscopePeriodResponse,
        vec![
            Op::Sanitize,
            Op::Typography,
            Op::NormalizeLength,
            Op::ReduceRepetition,
            Op::HumanizeLabels,
            Op::ValidateQuality,
            Op::BuildFallback,
        ],
        payload,
        context_with_word_limits(word_limits),
    )
}

pub fn reprocess_natal_theme(
    reading: &mut NatalReadingResponse,
    language: &str,
) -> Result<TextReprocessingFieldAudit, TextReprocessingApplicationError> {
    reprocess_natal_theme_with_context(reading, language, TextRetreatmentRequestContext::default())
}

pub fn reprocess_natal_theme_with_context(
    reading: &mut NatalReadingResponse,
    language: &str,
    context: TextRetreatmentRequestContext,
) -> Result<TextReprocessingFieldAudit, TextReprocessingApplicationError> {
    let original = serde_json::to_value(&*reading)?;
    let response = run(
        language,
        SERVICE_NATAL_THEME,
        TextTarget::NatalReading,
        vec![
            Op::Sanitize,
            Op::Typography,
            Op::HumanizeLabels,
            Op::ValidateQuality,
        ],
        original,
        context,
    );
    let audit = field_audit_from_response(&response);
    *reading = serde_json::from_value(response.payload)?;
    remove_chapter_symbolic_disclaimer_boilerplate(reading);
    Ok(audit)
}

pub fn normalize_json_for_text_reprocessing_parity(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(normalize_json_for_text_reprocessing_parity)
                .collect(),
        ),
        Value::Object(map) => {
            let mut normalized = serde_json::Map::new();
            for (key, value) in map {
                normalized.insert(
                    key.clone(),
                    normalize_json_for_text_reprocessing_parity(value),
                );
            }
            Value::Object(normalized)
        }
        Value::String(text) => Value::String(text.trim().to_string()),
        _ => value.clone(),
    }
}

fn run(
    language: &str,
    service: &str,
    target: TextTarget,
    operations: Vec<Op>,
    payload: Value,
    context: TextRetreatmentRequestContext,
) -> TextRetreatmentResponse {
    TextRetreatmentPipeline::default().process(TextRetreatmentRequest {
        language: TextLanguage::new(language),
        service: TextService::new(service),
        target,
        operations,
        payload,
        context,
    })
}

fn prompt_message_to_json(message: &PromptMessage) -> Value {
    let role = match message.role {
        PromptRole::System => "system",
        PromptRole::Developer => "developer",
        PromptRole::User => "user",
        PromptRole::Assistant => "assistant",
    };
    json!({
        "role": role,
        "content": message.content,
    })
}

fn context_with_word_limits(word_limits: Option<TextWordLimits>) -> TextRetreatmentRequestContext {
    TextRetreatmentRequestContext {
        word_limits,
        ..TextRetreatmentRequestContext::default()
    }
}

fn field_audit_from_response(response: &TextRetreatmentResponse) -> TextReprocessingFieldAudit {
    let mut audit = TextReprocessingFieldAudit::default();
    for item in &response.audit {
        let Some(path) = item.field_path.as_ref() else {
            continue;
        };
        let field = public_field_path(path);
        match item.operation {
            Op::Sanitize if item.action == TextRetreatmentAuditAction::Changed => {
                audit.sanitized_fields.push(field)
            }
            Op::Typography if item.action == TextRetreatmentAuditAction::Changed => {
                audit.typography_fields.push(field)
            }
            Op::BuildFallback if item.action == TextRetreatmentAuditAction::FallbackApplied => {
                audit.fallback_fields.push(field)
            }
            Op::ValidateQuality if item.action == TextRetreatmentAuditAction::Validated => {
                audit.validated_fields.push(field)
            }
            _ => {}
        }
    }
    audit
}

fn public_field_path(path: &str) -> String {
    path.trim_start_matches("$.")
        .trim_start_matches('$')
        .to_string()
}
