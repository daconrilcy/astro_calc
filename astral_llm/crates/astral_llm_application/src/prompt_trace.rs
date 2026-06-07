//! Journalisation du prompt exact envoye au provider LLM (tracing + fichiers).

use std::fs::{create_dir_all, write};
use std::path::PathBuf;

use astral_llm_domain::chapter_orchestration::READING_SUMMARY_STEP_CODE;
use astral_llm_infra::config::{env_bool, env_var};
use astral_llm_providers::{PromptMessage, PromptRole};

use crate::prompt_compiler::{PromptBundle, PromptCompiler};

pub const TARGET: &str = "astral_llm.prompt";
const DEFAULT_PROMPT_LOG_DIR: &str = "output/logs/prompts";

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
    if !env_bool("ASTRAL_LLM_LOG_COMPILED_PROMPTS", true) {
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
    if !env_bool("ASTRAL_LLM_LOG_COMPILED_PROMPTS", true) {
        return None;
    }
    let raw = env_var("ASTRAL_LLM_PROMPT_LOG_DIR").unwrap_or_else(|| DEFAULT_PROMPT_LOG_DIR.into());
    Some(PathBuf::from(raw))
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
    messages
        .iter()
        .map(|m| {
            let role = match m.role {
                PromptRole::System => "system",
                PromptRole::Developer => "developer",
                PromptRole::User => "user",
                PromptRole::Assistant => "assistant",
            };
            format!("<<< {role} >>>\n{}\n", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn formats_roles_in_order() {
        let text = format_compiled_messages(&[
            PromptMessage {
                role: PromptRole::System,
                content: "sys".into(),
            },
            PromptMessage {
                role: PromptRole::User,
                content: "usr".into(),
            },
        ]);
        assert!(text.contains("<<< system >>>\nsys"));
        assert!(text.contains("<<< user >>>\nusr"));
    }

    #[test]
    fn sanitize_replaces_invalid_chars() {
        assert_eq!(
            prompt_log_chapter_segment(Some(READING_SUMMARY_STEP_CODE)),
            "summary"
        );
        assert_eq!(sanitize_filename_segment("a/b"), "a_b");
    }

    #[test]
    fn writes_prompt_under_run_subdirectory() {
        let _guard = env_lock();
        let temp =
            std::env::temp_dir().join(format!("astral_prompt_log_test_{}", uuid::Uuid::new_v4()));
        std::env::set_var("ASTRAL_LLM_LOG_COMPILED_PROMPTS", "true");
        std::env::set_var("ASTRAL_LLM_PROMPT_LOG_DIR", temp.to_string_lossy().as_ref());

        let path = write_prompt_file(
            "run-abc",
            Some("identity"),
            Some("natal_premium"),
            Some("v1"),
            "primary",
            "<<< user >>>\nhello\n",
        )
        .expect("file written");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("run_id: run-abc"));
        assert!(content.contains("<<< user >>>\nhello"));

        let _ = std::fs::remove_dir_all(&temp);
        std::env::remove_var("ASTRAL_LLM_PROMPT_LOG_DIR");
    }
}
