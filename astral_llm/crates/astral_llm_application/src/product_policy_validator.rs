use astral_llm_domain::{
    GenerateReadingRequest, GenerationError, GenerationErrorCode, ProductGenerationPolicy,
    ProviderKind,
};

pub struct ProductPolicyValidator;

impl ProductPolicyValidator {
    pub fn validate<'a>(
        request: &GenerateReadingRequest,
        policy: &'a ProductGenerationPolicy,
        resolved_provider: &ProviderKind,
        resolved_model: &str,
    ) -> Result<&'a ProductGenerationPolicy, GenerationError> {
        Self::validate_against_policy(request, policy, resolved_provider, resolved_model)?;
        Ok(policy)
    }

    pub fn validate_against_policy(
        request: &GenerateReadingRequest,
        policy: &ProductGenerationPolicy,
        resolved_provider: &ProviderKind,
        resolved_model: &str,
    ) -> Result<(), GenerationError> {
        if !policy.allows_provider(resolved_provider) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                "provider not allowed for this product",
                serde_json::json!({
                    "product_code": policy.product_code,
                    "provider": resolved_provider.as_str()
                }),
            ));
        }

        if !policy.allows_model(resolved_provider, resolved_model) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                "model not allowed for this product",
                serde_json::json!({
                    "product_code": policy.product_code,
                    "model": resolved_model
                }),
            ));
        }

        if !policy.allows_mode(&request.response_contract.generation_mode) {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                "generation mode not allowed for this product",
                serde_json::json!({
                    "generation_mode": request.response_contract.generation_mode.as_str()
                }),
            ));
        }

        let domain_count = request
            .engine
            .domain_count
            .unwrap_or_else(|| {
                // default applied later via profile in domain resolver when natal_prompter
                3
            });
        if domain_count > policy.max_domains {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                format!("domain_count exceeds product maximum ({})", policy.max_domains),
                serde_json::json!({ "max_domains": policy.max_domains }),
            ));
        }

        let chapter_count = if request.response_contract.chapters.is_empty() {
            domain_count
        } else {
            request.response_contract.chapters.len() as u8
        };
        if chapter_count > policy.max_chapters {
            return Err(GenerationError::with_details(
                GenerationErrorCode::ProductPolicyViolation,
                format!("chapter count exceeds product maximum ({})", policy.max_chapters),
                serde_json::json!({ "max_chapters": policy.max_chapters }),
            ));
        }

        if let Some(tokens) = request.engine.max_output_tokens {
            if tokens > policy.max_output_tokens {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::ProductPolicyViolation,
                    "max_output_tokens exceeds product limit",
                    serde_json::json!({ "max_output_tokens": policy.max_output_tokens }),
                ));
            }
        }

        Ok(())
    }
}
