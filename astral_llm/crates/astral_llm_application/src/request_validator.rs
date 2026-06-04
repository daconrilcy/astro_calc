use astral_llm_domain::{
    GenerateReadingRequest, GenerationError, GenerationErrorCode, ProviderKind, ServiceLimits,
};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::payload_sanitizer::scan_json_for_injection;

pub struct RequestValidator;

impl RequestValidator {
    pub fn validate(
        request: &GenerateReadingRequest,
        limits: &ServiceLimits,
        catalog: &SharedCanonicalCatalog,
    ) -> Result<(), GenerationError> {
        let mut violations = Vec::new();

        if request.product_context.product_code.trim().is_empty() {
            violations.push("product_context.product_code is required".into());
        }

        if request.product_context.product_code == astral_llm_domain::NATAL_PROMPTER_PRODUCT {
            let profile_code = request
                .product_context
                .interpretation_profile_code
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            if profile_code.is_none() {
                violations.push(
                    "product_context.interpretation_profile_code is required for natal_prompter"
                        .into(),
                );
            } else if let Some(code) = profile_code {
                if catalog.interpretation_profile(code).is_none() {
                    violations.push(format!(
                        "interpretation profile not found or inactive: {code}"
                    ));
                }
            }
        }

        let lang = request.product_context.user_language.trim().to_lowercase();
        if lang.len() != 2 {
            violations.push("product_context.user_language must be a 2-letter ISO code".into());
        } else if !catalog.writing_locales.is_empty()
            && catalog.writing_locale(&lang).is_none()
        {
            violations.push(format!(
                "product_context.user_language '{lang}' is not an active writing locale"
            ));
        }

        if request.astro_result.contract_version.is_empty() {
            violations.push("astro_result.contract_version is required".into());
        }

        if request.response_contract.output_schema_version.is_empty() {
            violations.push("response_contract.output_schema_version is required".into());
        }

        if let Some(provider) = &request.engine.provider {
            if matches!(provider, ProviderKind::Custom(_)) {
                violations.push("engine.provider custom values are not supported".into());
            }
        }

        if let Some(count) = request.engine.domain_count {
            if count == 0 || count > limits.max_domain_count {
                violations.push(format!(
                    "engine.domain_count must be between 1 and {}",
                    limits.max_domain_count
                ));
            }
        }

        if let Some(temp) = request.engine.temperature {
            if !(0.0..=2.0).contains(&temp) {
                violations.push("engine.temperature must be between 0.0 and 2.0".into());
            }
        }

        if let Some(custom) = &request.astrologer_profile.custom_instructions {
            if custom.len() > limits.max_custom_instructions_chars {
                violations.push(format!(
                    "custom_instructions exceeds {} characters",
                    limits.max_custom_instructions_chars
                ));
            }
        }

        let astro_bytes = serde_json::to_vec(&request.astro_result.data).unwrap_or_default();
        if astro_bytes.len() > limits.max_astro_json_bytes {
            violations.push(format!(
                "astro_result.data exceeds {} bytes",
                limits.max_astro_json_bytes
            ));
        }

        if let Some(v) = scan_json_for_injection(&request.astro_result.data) {
            violations.push(v);
        }

        let allowed_domains = catalog.domains_or_fallback(&[]);
        if allowed_domains.is_empty() {
            violations.push("no astrological domains configured in canonical catalog".into());
        }

        if request.response_contract.output_schema_version != "natal_reading_v1"
            && request.response_contract.output_schema_version != "chapter_provider_v1"
        {
            violations.push(format!(
                "unsupported output_schema_version: {}",
                request.response_contract.output_schema_version
            ));
        }

        let chapter_count = if request.response_contract.chapters.is_empty() {
            request.engine.domain_count.unwrap_or(3) as usize
        } else {
            request.response_contract.chapters.len()
        };
        if chapter_count > limits.max_chapters_per_request as usize {
            violations.push(format!(
                "too many chapters requested (max {})",
                limits.max_chapters_per_request
            ));
        }

        if !violations.is_empty() {
            return Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                "request validation failed",
                serde_json::json!({ "violations": violations }),
            ));
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        astrologer_profile::{JargonLevel, ToneProfile, WordingStyle},
        engine_params::EngineParams,
        generation_request::{AudienceLevel, ProductContext},
        output_contract::{GenerationMode, OutputFormat, ResponseContract},
        provider::ProviderKind,
        AstroCalculationPayload, AstrologerProfile,
    };
    use astral_llm_infra::CanonicalCatalog;

    fn sample_request() -> GenerateReadingRequest {
        GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: ProductContext {
                product_code: "natal_prompter".into(),
                interpretation_profile_code: Some("natal_light".into()),
                user_language: "fr".into(),
                audience_level: AudienceLevel::Beginner,
            },
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v13".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({}),
            },
            astrologer_profile: AstrologerProfile {
                profile_id: None,
                name: None,
                tone: ToneProfile::Warm,
                jargon_level: JargonLevel::Beginner,
                wording_style: WordingStyle::Clear,
                preferred_domains: vec![],
                forbidden_wording: vec![],
                custom_instructions: None,
            },
            engine: EngineParams {
                provider: Some(ProviderKind::Fake),
                model: Some("fake".into()),
                reasoning_effort: None,
                temperature: None,
                max_output_tokens: None,
                domain_count: Some(3),
                allow_fallback: true,
                timeout_ms: None,
                allow_oracle_benchmark: false,
                summary_model: None,
            },
            response_contract: ResponseContract {
                output_schema_version: "natal_reading_v1".into(),
                generation_mode: GenerationMode::SinglePass,
                format: OutputFormat::StructuredJson,
                chapters: vec![],
                global_max_tokens: None,
                include_astro_sources: true,
                include_legal_disclaimer: true,
            },
            safety_policy: None,
        }
    }

    #[test]
    fn rejects_custom_provider() {
        let mut request = sample_request();
        request.engine.provider = Some(ProviderKind::Custom("evil".into()));
        let catalog = std::sync::Arc::new(CanonicalCatalog::default());
        assert!(RequestValidator::validate(&request, &ServiceLimits::default(), &catalog).is_err());
    }
}
