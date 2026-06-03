use astral_llm_domain::{
    GenerateReadingRequest, GenerateReadingResponse, GenerationError, GenerationErrorCode,
};

use crate::engine_defaults::ResolvedEngineParams;
use crate::execution_audit::ExecutionAudit;

pub const TARGET: &str = "astral_llm.generation";

#[derive(Debug, Clone)]
pub struct GenerationTraceContext {
    pub run_id: String,
    pub request_id: Option<String>,
    pub product_code: String,
    pub idempotency_key: Option<String>,
}

impl GenerationTraceContext {
    pub fn from_request(run_id: &str, request: &GenerateReadingRequest) -> Self {
        Self {
            run_id: run_id.to_string(),
            request_id: request.request_id.clone(),
            product_code: request.product_context.product_code.clone(),
            idempotency_key: request.idempotency_key.clone(),
        }
    }

    pub fn started(&self, engine: &ResolvedEngineParams, generation_mode: &str) {
        tracing::info!(
            target: TARGET,
            run_id = %self.run_id,
            request_id = self.request_id.as_deref().unwrap_or("-"),
            product_code = %self.product_code,
            idempotency_key = self.idempotency_key.as_deref().unwrap_or("-"),
            provider = engine.provider.as_str(),
            model = %engine.model,
            reasoning_effort = ?engine.reasoning_effort,
            generation_mode,
            "generation started"
        );
    }

    pub fn finished(
        &self,
        response: &GenerateReadingResponse,
        latency_ms: u64,
        audit: &ExecutionAudit,
    ) {
        let steps_generated = audit
            .steps
            .iter()
            .filter(|s| matches!(s.status.as_str(), "generated" | "repaired"))
            .count();
        let steps_failed = audit
            .steps
            .iter()
            .filter(|s| s.status.as_str() == "failed")
            .count();

        match response {
            GenerateReadingResponse::Success(success) => {
                tracing::info!(
                    target: TARGET,
                    run_id = %self.run_id,
                    request_id = self.request_id.as_deref().unwrap_or("-"),
                    product_code = %self.product_code,
                    latency_ms,
                    chapter_count = success.reading.chapters.len(),
                    steps_generated,
                    steps_failed,
                    selected_domains = ?audit.selected_domains,
                    "generation succeeded"
                );
            }
            GenerateReadingResponse::SafetyRejected(rejected) => {
                tracing::warn!(
                    target: TARGET,
                    run_id = %self.run_id,
                    request_id = self.request_id.as_deref().unwrap_or("-"),
                    product_code = %self.product_code,
                    latency_ms,
                    error_code = %rejected.error.code,
                    rule_id = rejected.error.rule_id.as_deref().unwrap_or("-"),
                    violations = ?rejected.violations,
                    steps_generated,
                    steps_failed,
                    "generation safety rejected"
                );
            }
            GenerateReadingResponse::Failed(failed) => {
                tracing::error!(
                    target: TARGET,
                    run_id = %self.run_id,
                    request_id = self.request_id.as_deref().unwrap_or("-"),
                    product_code = %self.product_code,
                    latency_ms,
                    error_code = failed.error.code.as_str(),
                    error_message = %failed.error.message,
                    error_details = ?failed.error.details,
                    steps_generated,
                    steps_failed,
                    selected_domains = ?audit.selected_domains,
                    audit_steps = ?audit.steps,
                    "generation failed"
                );
            }
        }
    }

    pub fn provider_failure(
        &self,
        provider: &str,
        model: &str,
        chapter_code: Option<&str>,
        error: &str,
    ) {
        tracing::warn!(
            target: "astral_llm.provider",
            run_id = %self.run_id,
            request_id = self.request_id.as_deref().unwrap_or("-"),
            product_code = %self.product_code,
            provider,
            model,
            chapter_code = chapter_code.unwrap_or("-"),
            error,
            "provider call failed"
        );
    }

    pub fn quality_failed(&self, error: &GenerationError) {
        let GenerationError::Detailed { detail, .. } = error;
        if detail.code == GenerationErrorCode::ReadingQualityFailed {
            tracing::warn!(
                target: TARGET,
                run_id = %self.run_id,
                request_id = self.request_id.as_deref().unwrap_or("-"),
                product_code = %self.product_code,
                error_code = detail.code.as_str(),
                error_details = ?detail.details,
                "reading quality below threshold"
            );
        }
    }
}
