use astral_llm_domain::{GenerateReadingRequest, ProductGenerationPolicy, ServiceLimits};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::domain_resolver::DomainResolver;

/// Alias historique — delegue a `DomainResolver`.
pub fn select_domains(
    request: &GenerateReadingRequest,
    catalog: &SharedCanonicalCatalog,
    limits: &ServiceLimits,
) -> Vec<String> {
    let policy = catalog
        .product_policy(&request.product_context.product_code)
        .cloned()
        .unwrap_or_else(ProductGenerationPolicy::bootstrap_basic);
    DomainResolver::resolve(request, catalog, limits, &policy)
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
    use std::sync::Arc;

    fn base_request(data: serde_json::Value) -> GenerateReadingRequest {
        GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: ProductContext {
                product_code: "natal_basic".into(),
                user_language: "fr".into(),
                audience_level: AudienceLevel::Beginner,
            },
            astro_result: AstroCalculationPayload {
                contract_version: "natal_structured_v13".into(),
                chart_type: "natal".into(),
                data,
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
                domain_count: Some(2),
                allow_fallback: true,
                timeout_ms: None,
                allow_oracle_benchmark: false,
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
    fn uses_domain_scores_when_present() {
        let catalog = Arc::new(CanonicalCatalog {
            astrological_domains: vec!["career".into(), "identity".into()],
            product_generation_policies: vec![ProductGenerationPolicy::bootstrap_basic()],
            ..Default::default()
        });
        let request = base_request(serde_json::json!({
            "domain_scores": { "career": 0.9, "identity": 0.5 }
        }));
        let domains = select_domains(&request, &catalog, &ServiceLimits::default());
        assert_eq!(domains[0], "career");
    }
}
