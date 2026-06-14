//! File-based storage for raw provider outputs before post-processing.

use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use astral_llm_domain::{AstralLlmEnv, ProviderKind};
use astral_llm_infra::config::{env_bool, env_var};
use astral_llm_providers::{ProviderGenerationRequest, ProviderGenerationResponse};
use serde_json::json;

const DEFAULT_RAW_OUTPUT_DIR: &str = "output/logs/raw_llm_outputs";
const MAX_FILENAME_SEGMENT_LEN: usize = 96;

pub const RAW_PROVIDER_TRACE_ENV: &str = "ASTRAL_LLM_STORE_RAW_PROVIDER_OUTPUTS";
pub const RAW_PROVIDER_TRACE_DIR_ENV: &str = "ASTRAL_LLM_RAW_PROVIDER_OUTPUT_DIR";

pub fn log_raw_provider_response(
    request: &ProviderGenerationRequest,
    provider: &ProviderKind,
    response: &ProviderGenerationResponse,
    fallback_used: bool,
) -> Option<PathBuf> {
    let base = raw_provider_trace_base_dir()?;
    write_raw_provider_response(&base, request, provider, response, fallback_used)
}

fn write_raw_provider_response(
    base: &std::path::Path,
    request: &ProviderGenerationRequest,
    provider: &ProviderKind,
    response: &ProviderGenerationResponse,
    fallback_used: bool,
) -> Option<PathBuf> {
    let run_id = sanitize_filename_segment(&request.metadata.run_id);
    let chapter = request
        .metadata
        .chapter_code
        .as_deref()
        .map(sanitize_filename_segment)
        .unwrap_or_else(|| "full".into());
    let dir = base.join(&run_id);
    if let Err(err) = create_dir_all(&dir) {
        tracing::warn!(
            run_id = %request.metadata.run_id,
            error = %err,
            "failed to create raw provider output directory"
        );
        return None;
    }

    let provider_segment = sanitize_filename_segment(provider.as_str());
    let trace_id = uuid::Uuid::new_v4();
    let created_at_epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0);
    let path = dir.join(format!(
        "{chapter}_{provider_segment}_{created_at_epoch_ms}_{trace_id}_raw_provider_response.json"
    ));
    let payload = json!({
        "trace_id": trace_id.to_string(),
        "created_at_epoch_ms": created_at_epoch_ms,
        "run_id": request.metadata.run_id,
        "request_id": request.metadata.request_id,
        "product_code": request.metadata.product_code,
        "chapter_code": request.metadata.chapter_code,
        "requested_model": request.model,
        "provider": provider.as_str(),
        "model_used": response.model_used,
        "fallback_used": fallback_used,
        "raw_text": response.raw_text,
        "parsed_json": response.parsed_json,
        "provider_metadata": response.provider_metadata,
        "usage": response.usage.as_ref().map(|usage| json!({
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
        })),
    });
    let body = match serde_json::to_string_pretty(&payload) {
        Ok(body) => body,
        Err(err) => {
            tracing::warn!(
                run_id = %request.metadata.run_id,
                error = %err,
                "failed to serialize raw provider output"
            );
            return None;
        }
    };
    match write(&path, body) {
        Ok(()) => Some(path),
        Err(err) => {
            tracing::warn!(
                run_id = %request.metadata.run_id,
                path = %path.display(),
                error = %err,
                "failed to write raw provider output"
            );
            None
        }
    }
}

pub fn raw_provider_trace_base_dir() -> Option<PathBuf> {
    if !raw_provider_trace_enabled() {
        return None;
    }
    let raw = env_var(RAW_PROVIDER_TRACE_DIR_ENV).unwrap_or_else(|| DEFAULT_RAW_OUTPUT_DIR.into());
    Some(PathBuf::from(raw))
}

fn raw_provider_trace_enabled() -> bool {
    let runtime_env = env_var("ASTRAL_LLM_ENV")
        .map(|value| AstralLlmEnv::parse(&value))
        .unwrap_or(AstralLlmEnv::Local);
    env_bool(RAW_PROVIDER_TRACE_ENV, !runtime_env.is_production())
}

fn sanitize_filename_segment(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "unknown".into();
    }
    let mut sanitized: String = trimmed
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    sanitized.truncate(MAX_FILENAME_SEGMENT_LEN);
    sanitized
}
