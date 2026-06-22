//! Journalisation du prompt exact envoye au provider LLM (tracing + fichiers).

use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use astral_llm_domain::chapter_orchestration::READING_SUMMARY_STEP_CODE;
use astral_llm_providers::{PromptMessage, PromptRole, ProviderGenerationRequest};
use serde_json::{json, Value};

use crate::prompt_compiler::{PromptBundle, PromptCompiler};
use crate::text_reprocessing_service_adapter::reprocess_prompt_trace;

pub const TARGET: &str = "astral_llm.prompt";
const DEFAULT_PROMPT_LOG_DIR: &str = "output/logs/prompts";
static PROMPT_TRACE_SETTINGS: OnceLock<RwLock<PromptTraceSettings>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct PromptTraceSettings {
    pub enabled: bool,
    pub log_dir: PathBuf,
}

impl Default for PromptTraceSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            log_dir: PathBuf::from(DEFAULT_PROMPT_LOG_DIR),
        }
    }
}

impl PromptTraceSettings {
    pub fn from_runtime(enabled: bool, log_dir: Option<PathBuf>) -> Self {
        let mut settings = Self {
            enabled,
            ..Self::default()
        };
        if let Some(log_dir) = log_dir {
            settings.log_dir = log_dir;
        }
        settings
    }
}

#[derive(Debug, Clone)]
pub struct PromptTraceRecord {
    pub chapter_code: Option<String>,
    pub step_type: Option<String>,
    pub attempt: Option<String>,
    pub prompt_family: Option<String>,
    pub prompt_version: Option<String>,
    pub message_count: i32,
    pub compiled_prompt: String,
    pub messages_json: Value,
}

pub fn configure_prompt_trace(settings: PromptTraceSettings) {
    *prompt_trace_settings_cell()
        .write()
        .expect("prompt trace settings lock poisoned") = settings;
}

pub fn log_prompt_bundle(
    run_id: &str,
    chapter_code: Option<&str>,
    bundle: &PromptBundle,
    compiler: &PromptCompiler,
    attempt: Option<&str>,
) {
    let messages = compiler.to_provider_messages(bundle);
    log_provider_messages(
        run_id,
        chapter_code,
        Some(bundle.prompt_family.as_str()),
        Some(bundle.prompt_version.as_str()),
        attempt,
        &messages,
    );
}

pub fn log_provider_messages(
    run_id: &str,
    chapter_code: Option<&str>,
    prompt_family: Option<&str>,
    prompt_version: Option<&str>,
    attempt: Option<&str>,
    messages: &[PromptMessage],
) {
    let settings = current_prompt_trace_settings();
    if !settings.enabled {
        return;
    }

    let compiled = format_compiled_messages(messages);
    let attempt_label = attempt.unwrap_or("primary");

    tracing::debug!(
        target: TARGET,
        run_id = %run_id,
        chapter_code = chapter_code.unwrap_or("-"),
        prompt_family = prompt_family.unwrap_or("-"),
        prompt_version = prompt_version.unwrap_or("-"),
        attempt = attempt_label,
        message_count = messages.len(),
        char_len = compiled.len(),
        compiled_prompt = %compiled,
        "compiled prompt sent to LLM"
    );

    if let Some(path) = write_prompt_file(
        run_id,
        chapter_code,
        prompt_family,
        prompt_version,
        attempt_label,
        &compiled,
    ) {
        tracing::debug!(
            target: TARGET,
            run_id = %run_id,
            chapter_code = chapter_code.unwrap_or("-"),
            attempt = attempt_label,
            path = %path.display(),
            "compiled prompt written to file"
        );
    }
}

pub fn build_prompt_trace_record(request: &ProviderGenerationRequest) -> PromptTraceRecord {
    PromptTraceRecord {
        chapter_code: request.metadata.chapter_code.clone(),
        step_type: request.metadata.prompt_trace_step.clone(),
        attempt: request.metadata.prompt_trace_attempt.clone(),
        prompt_family: request.metadata.prompt_family.clone(),
        prompt_version: request.metadata.prompt_version.clone(),
        message_count: i32::try_from(request.messages.len()).unwrap_or(i32::MAX),
        compiled_prompt: format_compiled_messages(&request.messages),
        messages_json: Value::Array(
            request
                .messages
                .iter()
                .map(prompt_message_to_json)
                .collect::<Vec<_>>(),
        ),
    }
}

fn write_prompt_file(
    run_id: &str,
    chapter_code: Option<&str>,
    prompt_family: Option<&str>,
    prompt_version: Option<&str>,
    attempt: &str,
    compiled: &str,
) -> Option<PathBuf> {
    let base = prompt_log_base_dir()?;
    let chapter_seg = prompt_log_chapter_segment(chapter_code);
    let attempt_seg = sanitize_filename_segment(attempt);
    let dir = base.join(sanitize_filename_segment(run_id));
    if let Err(err) = create_dir_all(&dir) {
        tracing::warn!(
            target: TARGET,
            run_id = %run_id,
            error = %err,
            "failed to create prompt log directory"
        );
        return None;
    }

    let path = dir.join(format!("{chapter_seg}_{attempt_seg}.txt"));
    let header = format!(
        "# run_id: {run_id}\n\
         # chapter_code: {}\n\
         # prompt_family: {}\n\
         # prompt_version: {}\n\
         # attempt: {attempt}\n\n",
        chapter_code.unwrap_or("-"),
        prompt_family.unwrap_or("-"),
        prompt_version.unwrap_or("-"),
    );
    let body = format!("{header}{compiled}");
    match write(&path, body) {
        Ok(()) => Some(path),
        Err(err) => {
            tracing::warn!(
                target: TARGET,
                run_id = %run_id,
                path = %path.display(),
                error = %err,
                "failed to write compiled prompt file"
            );
            None
        }
    }
}

pub fn prompt_log_base_dir() -> Option<PathBuf> {
    let settings = current_prompt_trace_settings();
    if !settings.enabled {
        return None;
    }
    Some(settings.log_dir)
}

fn prompt_trace_settings_cell() -> &'static RwLock<PromptTraceSettings> {
    PROMPT_TRACE_SETTINGS.get_or_init(|| RwLock::new(PromptTraceSettings::default()))
}

fn current_prompt_trace_settings() -> PromptTraceSettings {
    prompt_trace_settings_cell()
        .read()
        .expect("prompt trace settings lock poisoned")
        .clone()
}

fn prompt_log_chapter_segment(chapter_code: Option<&str>) -> String {
    match chapter_code {
        Some(READING_SUMMARY_STEP_CODE) => "summary".into(),
        Some(code) => sanitize_filename_segment(code),
        None => "full".into(),
    }
}

fn sanitize_filename_segment(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "unknown".into();
    }
    trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn format_compiled_messages(messages: &[PromptMessage]) -> String {
    reprocess_prompt_trace(messages)
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
