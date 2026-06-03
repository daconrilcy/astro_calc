use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use astral_llm_domain::{
    provider::{ProviderKind, StructuredOutputMode},
    FallbackPolicy, FallbackReason, GenerationError, GenerationErrorCode, PrivacyPolicy,
};
use astral_llm_providers::{
    LlmProvider, LlmProviderError, ProviderGenerationRequest, ProviderGenerationResponse,
    SharedLlmProvider,
};

use crate::model_capability_registry::ModelCapabilityRegistry;
use crate::provider_circuit_breaker::{CircuitBreakerState, ProviderCircuitBreaker};

#[derive(Debug, Clone)]
pub struct ProviderRouteResult {
    pub response: ProviderGenerationResponse,
    pub requested_provider: ProviderKind,
    pub used_provider: ProviderKind,
    pub fallback_used: bool,
}

pub struct ProviderRouter {
    providers: HashMap<ProviderKind, SharedLlmProvider>,
    fallback_policy: FallbackPolicy,
    capability_registry: Arc<ModelCapabilityRegistry>,
    privacy_policy: PrivacyPolicy,
    circuit_breaker: Arc<ProviderCircuitBreaker>,
}

impl ProviderRouter {
    pub fn new(
        providers: HashMap<ProviderKind, SharedLlmProvider>,
        fallback_policy: FallbackPolicy,
        capability_registry: Arc<ModelCapabilityRegistry>,
        privacy_policy: PrivacyPolicy,
        circuit_breaker: Arc<ProviderCircuitBreaker>,
    ) -> Self {
        Self {
            providers,
            fallback_policy,
            capability_registry,
            privacy_policy,
            circuit_breaker,
        }
    }

    pub fn circuit_breaker(&self) -> &ProviderCircuitBreaker {
        &self.circuit_breaker
    }

    pub fn circuit_states(&self) -> Vec<(String, CircuitBreakerState)> {
        self.circuit_breaker.snapshot()
    }

    pub fn capability_registry(&self) -> &ModelCapabilityRegistry {
        &self.capability_registry
    }

    pub fn list_model_capabilities(&self) -> Vec<astral_llm_domain::ModelCapability> {
        self.capability_registry
            .list_active()
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn generate(
        &self,
        request: ProviderGenerationRequest,
        requested_provider: ProviderKind,
        requested_model: &str,
        allow_fallback: bool,
        require_strict_schema: bool,
    ) -> Result<ProviderRouteResult, GenerationError> {
        if matches!(requested_provider, ProviderKind::Custom(_)) {
            return Err(GenerationError::new(
                GenerationErrorCode::UnsupportedProvider,
                "custom providers are not supported",
            ));
        }

        self.capability_registry.validate_request_capabilities(
            &requested_provider,
            requested_model,
            request.reasoning_effort,
            require_strict_schema,
        )?;

        let required_cap = self
            .capability_registry
            .require(&requested_provider, requested_model)?;

        let candidates = self.fallback_policy.candidate_chain(&requested_provider, allow_fallback);
        let mut last_error: Option<GenerationError> = None;

        for provider_kind in candidates {
            let circuit_allowed = self.circuit_breaker.allows_call(&provider_kind);
            if !circuit_allowed {
                tracing::warn!(
                    provider = provider_kind.as_str(),
                    "provider circuit open, skipping"
                );
                last_error = Some(GenerationError::with_details(
                    GenerationErrorCode::ProviderUnavailable,
                    "LLM provider temporarily unavailable",
                    serde_json::json!({
                        "provider": provider_kind.as_str(),
                        "circuit": "open"
                    }),
                ));
                continue;
            }

            let Some(provider) = self.providers.get(&provider_kind) else {
                self.circuit_breaker.release_half_open_probe(&provider_kind);
                continue;
            };

            let model_for_call = if provider_kind == requested_provider {
                requested_model.to_string()
            } else {
                self.capability_registry
                    .default_model_for_provider(&provider_kind)
                    .unwrap_or_else(|| requested_model.to_string())
            };

            if provider_kind != requested_provider {
                if !self.privacy_policy.allow_cross_provider_fallback
                    && !self.fallback_policy.allow_cross_vendor_data_transfer
                {
                    self.circuit_breaker.release_half_open_probe(&provider_kind);
                    last_error = Some(GenerationError::new(
                        GenerationErrorCode::PolicyViolation,
                        "cross-provider fallback is disabled by privacy policy",
                    ));
                    continue;
                }

                if !self.capability_registry.fallback_compatible(
                    required_cap,
                    &provider_kind,
                    &model_for_call,
                    self.fallback_policy.require_same_structured_output_level,
                ) {
                    self.circuit_breaker.release_half_open_probe(&provider_kind);
                    last_error = Some(GenerationError::with_details(
                        GenerationErrorCode::UnsupportedCapability,
                        "fallback provider lacks required structured output capabilities",
                        serde_json::json!({
                            "requested_provider": requested_provider.as_str(),
                            "fallback_provider": provider_kind.as_str(),
                            "model": requested_model
                        }),
                    ));
                    continue;
                }
            }

            if require_strict_schema
                && provider.capabilities().structured_output
                    != StructuredOutputMode::JsonSchemaStrict
            {
                self.circuit_breaker.release_half_open_probe(&provider_kind);
                last_error = Some(GenerationError::with_details(
                    GenerationErrorCode::UnsupportedCapability,
                    "provider does not support strict JSON schema output",
                    serde_json::json!({
                        "provider": provider_kind.as_str(),
                        "required_capability": "JsonSchemaStrict"
                    }),
                ));
                continue;
            }

            let mut provider_request = request.clone();
            provider_request.model = model_for_call.clone();

            let mut attempt = 0;
            while attempt <= self.fallback_policy.max_retries_per_provider {
                match provider.generate(provider_request.clone()).await {
                    Ok(response) => {
                        self.circuit_breaker.record_success(&provider_kind);
                        let fallback_used = provider_kind != requested_provider;
                        return Ok(ProviderRouteResult {
                            response,
                            requested_provider: requested_provider.clone(),
                            used_provider: provider_kind.clone(),
                            fallback_used,
                        });
                    }
                    Err(err) if err.is_transient() && attempt < self.fallback_policy.max_retries_per_provider => {
                        attempt += 1;
                        tracing::warn!(
                            provider = provider_kind.as_str(),
                            attempt,
                            error = %err,
                            "transient provider error, retrying"
                        );
                    }
                    Err(err) => {
                        if err.is_transient() {
                            self.circuit_breaker
                                .record_transient_failure(&provider_kind);
                        } else {
                            self.circuit_breaker.release_half_open_probe(&provider_kind);
                        }
                        tracing::warn!(
                            run_id = %request.metadata.run_id,
                            request_id = request.metadata.request_id.as_deref().unwrap_or("-"),
                            product_code = %request.metadata.product_code,
                            chapter_code = request.metadata.chapter_code.as_deref().unwrap_or("-"),
                            provider = provider_kind.as_str(),
                            error = %err,
                            "provider call failed"
                        );
                        let fallback_reason = map_fallback_reason(&err);
                        last_error = Some(map_provider_error(err, provider_kind.clone()));
                        if !self.fallback_policy.allows_fallback_for(fallback_reason) {
                            break;
                        }
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::UnsupportedProvider,
                "no provider available for request",
            )
        }))
    }
}

fn map_fallback_reason(err: &LlmProviderError) -> FallbackReason {
    match err {
        LlmProviderError::Timeout => FallbackReason::Timeout,
        LlmProviderError::RateLimited => FallbackReason::RateLimited,
        _ => FallbackReason::ProviderUnavailable,
    }
}

fn map_provider_error(err: LlmProviderError, provider: ProviderKind) -> GenerationError {
    let code = match &err {
        LlmProviderError::Timeout => GenerationErrorCode::ProviderTimeout,
        LlmProviderError::RateLimited => GenerationErrorCode::ProviderRateLimited,
        LlmProviderError::Http(_) | LlmProviderError::Api(_) => {
            GenerationErrorCode::ProviderUnavailable
        }
        LlmProviderError::InvalidResponse(_) => GenerationErrorCode::InvalidJsonOutput,
        LlmProviderError::Config(_) => GenerationErrorCode::UnsupportedProvider,
    };

    GenerationError::with_details(
        code,
        client_error_message(&code),
        serde_json::json!({ "provider": provider.as_str() }),
    )
}

fn client_error_message(code: &GenerationErrorCode) -> &'static str {
    match code {
        GenerationErrorCode::ProviderTimeout => "LLM provider request timed out",
        GenerationErrorCode::ProviderRateLimited => "LLM provider rate limit exceeded",
        GenerationErrorCode::ProviderUnavailable => "LLM provider temporarily unavailable",
        GenerationErrorCode::InvalidJsonOutput => "LLM provider returned invalid JSON",
        GenerationErrorCode::UnsupportedProvider => "requested LLM provider is not available",
        _ => "LLM generation failed",
    }
}

pub fn build_provider_map(
    providers: Vec<Arc<dyn LlmProvider>>,
) -> HashMap<ProviderKind, SharedLlmProvider> {
    providers
        .into_iter()
        .map(|p| (p.kind(), p))
        .collect()
}

pub fn build_http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("valid reqwest client")
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_providers::{FakeProvider, GenerationMetadata, PromptMessage, PromptRole};

    #[tokio::test]
    async fn routes_to_fake_provider() {
        let map = build_provider_map(vec![Arc::new(FakeProvider)]);
        let registry = Arc::new(ModelCapabilityRegistry::bootstrap());
        let router = ProviderRouter::new(
            map,
            FallbackPolicy::disabled(),
            registry,
            PrivacyPolicy::default(),
            Arc::new(ProviderCircuitBreaker::new(5, 60)),
        );

        let request = ProviderGenerationRequest {
            model: "fake-model".into(),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                content: "hello".into(),
            }],
            structured_schema: None,
            reasoning_effort: None,
            temperature: None,
            max_output_tokens: Some(500),
            safety_mode: astral_llm_domain::SafetyMode::PlatformRulesOnly,
            timeout: Duration::from_secs(30),
            metadata: GenerationMetadata {
                run_id: "r1".into(),
                request_id: None,
                product_code: "natal_basic".into(),
                chapter_code: None,
            },
        };

        let result = router
            .generate(
                request,
                ProviderKind::Fake,
                "fake-model",
                false,
                false,
            )
            .await
            .expect("ok");
        assert!(!result.fallback_used);
        assert!(result.response.parsed_json.is_some());
    }
}
