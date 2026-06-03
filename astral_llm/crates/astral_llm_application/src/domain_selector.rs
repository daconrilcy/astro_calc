use astral_llm_domain::{
    domain_selection::{DomainSelection, DomainSelectionStrategy},
    GenerateReadingRequest, ServiceLimits,
};
use astral_llm_infra::SharedCanonicalCatalog;

pub fn select_domains(
    request: &GenerateReadingRequest,
    catalog: &SharedCanonicalCatalog,
    limits: &ServiceLimits,
) -> Vec<String> {
    let max_count = limits.max_domain_count as usize;
    let count = request
        .engine
        .domain_count
        .unwrap_or(3)
        .max(1)
        .min(limits.max_domain_count) as usize;
    let preferred = &request.astrologer_profile.preferred_domains;

    if !preferred.is_empty() {
        return preferred.iter().take(count).cloned().collect();
    }

    let allowed = catalog.domains_or_fallback(&[]);
    let selection = DomainSelection {
        domain_count: count as u8,
        allowed_domains: allowed,
        selected_domains: None,
        selection_strategy: DomainSelectionStrategy::TopWeightedAstroSignals,
    };

    select_from_astro_signals(request, &selection, max_count)
}

fn select_from_astro_signals(
    request: &GenerateReadingRequest,
    selection: &DomainSelection,
    max_count: usize,
) -> Vec<String> {
    if let Some(explicit) = &selection.selected_domains {
        return explicit.iter().take(max_count).cloned().collect();
    }

    if let Some(scores) = request.astro_result.data.get("domain_scores").and_then(|v| v.as_object())
    {
        let mut ranked: Vec<(String, f64)> = scores
            .iter()
            .filter_map(|(k, v)| v.as_f64().map(|score| (k.clone(), score)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let domains: Vec<String> = ranked
            .into_iter()
            .map(|(k, _)| k)
            .filter(|d| selection.allowed_domains.contains(d))
            .take(selection.domain_count as usize)
            .collect();

        if !domains.is_empty() {
            return domains;
        }
    }

    selection
        .allowed_domains
        .iter()
        .take(selection.domain_count as usize)
        .cloned()
        .collect()
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
            ..Default::default()
        });
        let request = base_request(serde_json::json!({
            "domain_scores": { "career": 0.9, "identity": 0.5 }
        }));
        let domains = select_domains(&request, &catalog, &ServiceLimits::default());
        assert_eq!(domains[0], "career");
    }
}
