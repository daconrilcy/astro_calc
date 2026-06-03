use std::time::Duration;

use astral_llm_domain::{
    provider::{ProviderKind, StructuredOutputMode},
    GenerationError, GenerationErrorCode,
};
use astral_llm_providers::{
    LlmProvider, LlmProviderError, ProviderGenerationRequest, ProviderGenerationResponse,
    SharedLlmProvider,
};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    pub fallback_order: Vec<ProviderKind>,
    pub max_retries: u8,
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            fallback_order: vec![
                ProviderKind::OpenAi,
                ProviderKind::Mistral,
                ProviderKind::Anthropic,
            ],
            max_retries: 1,
        }
    }
}

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
}

impl ProviderRouter {
    pub fn new(
        providers: HashMap<ProviderKind, SharedLlmProvider>,
        fallback_policy: FallbackPolicy,
    ) -> Self {
        Self {
            providers,
            fallback_policy,
        }
    }

    pub fn provider_capabilities(
        &self,
    ) -> Vec<(ProviderKind, astral_llm_domain::ProviderCapabilities)> {
        self.providers
            .iter()
            .map(|(kind, provider)| (kind.clone(), provider.capabilities()))
            .collect()
    }

    pub async fn generate(
        &self,
        request: ProviderGenerationRequest,
        requested_provider: ProviderKind,
        allow_fallback: bool,
        require_strict_schema: bool,
    ) -> Result<ProviderRouteResult, GenerationError> {
        if matches!(requested_provider, ProviderKind::Custom(_)) {
            return Err(GenerationError::new(
                GenerationErrorCode::UnsupportedProvider,
                "custom providers are not supported",
            ));
        }

        let candidates = self.build_candidate_chain(requested_provider.clone(), allow_fallback);
        let mut last_error: Option<GenerationError> = None;

        for provider_kind in candidates {
            let Some(provider) = self.providers.get(&provider_kind) else {
                continue;
            };

            if require_strict_schema
                && provider.capabilities().structured_output
                    != StructuredOutputMode::JsonSchemaStrict
            {
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

            let mut attempt = 0;
            while attempt <= self.fallback_policy.max_retries {
                match provider.generate(request.clone()).await {
                    Ok(response) => {
                        let fallback_used = provider_kind != requested_provider;
                        return Ok(ProviderRouteResult {
                            response,
                            requested_provider: requested_provider.clone(),
                            used_provider: provider_kind.clone(),
                            fallback_used,
                        });
                    }
                    Err(err) if err.is_transient() && attempt < self.fallback_policy.max_retries => {
                        attempt += 1;
                        tracing::warn!(
                            provider = provider_kind.as_str(),
                            attempt,
                            error = %err,
                            "transient provider error, retrying"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            provider = provider_kind.as_str(),
                            error = %err,
                            "provider call failed"
                        );
                        last_error = Some(map_provider_error(err, provider_kind));
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

    fn build_candidate_chain(
        &self,
        requested: ProviderKind,
        allow_fallback: bool,
    ) -> Vec<ProviderKind> {
        let mut chain = vec![requested.clone()];
        if allow_fallback {
            for kind in &self.fallback_policy.fallback_order {
                if *kind != requested && !chain.contains(kind) {
                    chain.push(kind.clone());
                }
            }
        }
        chain
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
        let router = ProviderRouter::new(map, FallbackPolicy::default());

        let request = ProviderGenerationRequest {
            model: "fake".into(),
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
            .generate(request, ProviderKind::Fake, false, false)
            .await
            .expect("ok");
        assert!(!result.fallback_used);
        assert!(result.response.parsed_json.is_some());
    }
}
