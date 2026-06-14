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

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::provider::SafetyMode;
    use astral_llm_providers::{GenerationMetadata, ProviderGenerationRequest, TokenUsage};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|err| err.into_inner())
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn raw_provider_trace_is_enabled_by_default_outside_production() {
        let _guard = env_lock();
        let _env_guard = EnvGuard::set("ASTRAL_LLM_ENV", "local");
        let _trace_guard = EnvGuard::remove(RAW_PROVIDER_TRACE_ENV);

        assert!(raw_provider_trace_enabled());
        std::env::set_var("ASTRAL_LLM_ENV", "production");
        assert!(!raw_provider_trace_enabled());
        std::env::set_var(RAW_PROVIDER_TRACE_ENV, "true");
        assert!(raw_provider_trace_enabled());
        std::env::set_var(RAW_PROVIDER_TRACE_ENV, "false");
        assert!(!raw_provider_trace_enabled());
    }

    #[test]
    fn writes_raw_provider_response_file() {
        let _guard = env_lock();
        let _env_guard = EnvGuard::set("ASTRAL_LLM_ENV", "local");
        let _trace_guard = EnvGuard::set(RAW_PROVIDER_TRACE_ENV, "true");
        let dir = std::env::temp_dir().join(format!(
            "astral_raw_provider_trace_{}",
            uuid::Uuid::new_v4()
        ));
        let _dir_guard = EnvGuard::set(RAW_PROVIDER_TRACE_DIR_ENV, dir.to_string_lossy().as_ref());

        let request = ProviderGenerationRequest {
            model: "gpt-test".into(),
            messages: Vec::new(),
            structured_schema: None,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: None,
            safety_mode: SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_secs(1),
            metadata: GenerationMetadata {
                run_id: "run/raw:test".into(),
                request_id: Some("req-1".into()),
                product_code: "horoscope".into(),
                chapter_code: None,
            },
        };
        let response = ProviderGenerationResponse {
            raw_text: "{\"text\":\"brut\"}".into(),
            parsed_json: Some(json!({ "text": "brut" })),
            usage: Some(TokenUsage {
                input_tokens: 1,
                output_tokens: 2,
            }),
            provider_metadata: json!({ "id": "provider-response" }),
            model_used: "gpt-test".into(),
            provider_kind: ProviderKind::OpenAi,
        };

        let path =
            write_raw_provider_response(&dir, &request, &ProviderKind::OpenAi, &response, false)
                .unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"raw_text\""));
        assert!(content.contains("\"trace_id\""));
        assert!(content.contains("run/raw:test"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn sanitize_filename_segment_caps_long_values() {
        let sanitized = sanitize_filename_segment(&"x".repeat(MAX_FILENAME_SEGMENT_LEN + 50));
        assert_eq!(sanitized.len(), MAX_FILENAME_SEGMENT_LEN);
    }
}
