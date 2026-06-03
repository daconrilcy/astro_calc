use astral_llm_domain::{
    GenerateReadingRequest, GenerationError, GenerationErrorCode, ProductGenerationPolicy,
    ProviderKind,
};
use astral_llm_infra::SharedCanonicalCatalog;

pub struct ProductPolicyValidator;

impl ProductPolicyValidator {
    pub fn validate<'a>(
        request: &GenerateReadingRequest,
        catalog: &'a SharedCanonicalCatalog,
        resolved_provider: &ProviderKind,
        resolved_model: &str,
    ) -> Result<&'a ProductGenerationPolicy, GenerationError> {
        let policy = catalog
            .product_policy(&request.product_context.product_code)
            .ok_or_else(|| {
                GenerationError::with_details(
                    GenerationErrorCode::ProductPolicyViolation,
                    format!(
                        "no generation policy for product: {}",
                        request.product_context.product_code
                    ),
                    serde_json::json!({ "product_code": request.product_context.product_code }),
                )
            })?;

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
            return Err(GenerationError::new(
                GenerationErrorCode::ProductPolicyViolation,
                "generation mode not allowed for this product",
            ));
        }

        let domain_count = request.engine.domain_count.unwrap_or(3);
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

        Ok(policy)
    }
}
