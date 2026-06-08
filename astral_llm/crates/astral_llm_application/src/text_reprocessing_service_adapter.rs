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
    Ok(audit)
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
