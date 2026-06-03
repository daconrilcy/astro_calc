use astral_llm_domain::{GenerateReadingRequest, NatalReadingResponse, SafetyPolicy};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::payload_sanitizer::contains_prompt_injection;

pub struct SafetyGuard;

impl SafetyGuard {
    pub fn validate_request(
        request: &GenerateReadingRequest,
        policy: &SafetyPolicy,
        catalog: &SharedCanonicalCatalog,
    ) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();

        if request.astro_result.contract_version.is_empty() {
            violations.push("astro_result.contract_version is required".into());
        }
        if request.response_contract.output_schema_version.is_empty() {
            violations.push("response_contract.output_schema_version is required".into());
        }

        if let Some(custom) = &request.astrologer_profile.custom_instructions {
            if contains_unsafe_override(custom, catalog) {
                violations.push(
                    "custom_instructions attempt to override platform or safety rules".into(),
                );
            }
        }

        if !policy.require_disclaimer && request.response_contract.include_legal_disclaimer {
            violations.push("legal disclaimer required by contract but disabled by policy".into());
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn validate_response(
        reading: &NatalReadingResponse,
        policy: &SafetyPolicy,
        forbidden_wording: &[String],
        catalog: &SharedCanonicalCatalog,
    ) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();
        let corpus = collect_text(reading);

        if policy.forbid_medical_advice
            && matches_patterns(&corpus, &catalog.patterns_for_type("medical"))
        {
            violations.push("medical advice detected".into());
        }
        if policy.forbid_legal_advice
            && matches_patterns(&corpus, &catalog.patterns_for_type("legal"))
        {
            violations.push("legal advice detected".into());
        }
        if policy.forbid_financial_advice
            && matches_patterns(&corpus, &catalog.patterns_for_type("financial"))
        {
            violations.push("financial advice detected".into());
        }
        if policy.forbid_death_prediction
            && matches_patterns(&corpus, &catalog.patterns_for_type("death"))
        {
            violations.push("death prediction detected".into());
        }
        if policy.forbid_pregnancy_prediction
            && matches_patterns(&corpus, &catalog.patterns_for_type("pregnancy"))
        {
            violations.push("pregnancy prediction detected".into());
        }
        if policy.forbid_deterministic_claims
            && matches_patterns(&corpus, &catalog.patterns_for_type("deterministic"))
        {
            violations.push("deterministic claim detected".into());
        }

        if policy.require_symbolic_framing
            && !matches_patterns(&corpus, &catalog.patterns_for_type("symbolic"))
        {
            violations.push("missing symbolic/interpretive framing".into());
        }

        for topic in &policy.custom_forbidden_topics {
            if corpus.to_lowercase().contains(&topic.to_lowercase()) {
                violations.push(format!("forbidden topic detected: {topic}"));
            }
        }
        for word in forbidden_wording {
            if corpus.to_lowercase().contains(&word.to_lowercase()) {
                violations.push(format!("forbidden wording detected: {word}"));
            }
        }

        if policy.require_disclaimer && reading.legal.disclaimer.trim().is_empty() {
            violations.push("missing legal disclaimer".into());
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

fn collect_text(reading: &NatalReadingResponse) -> String {
    let mut parts = vec![
        reading.summary.title.clone(),
        reading.summary.short_text.clone(),
        reading.legal.disclaimer.clone(),
    ];
    for chapter in &reading.chapters {
        parts.push(chapter.title.clone());
        parts.push(chapter.body.clone());
    }
    parts.join("\n")
}

fn contains_unsafe_override(text: &str, catalog: &SharedCanonicalCatalog) -> bool {
    contains_prompt_injection(text)
        || matches_patterns(text, &catalog.patterns_for_type("injection"))
}

fn matches_patterns(text: &str, patterns: &[&str]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let lower = text.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        generation_response::{LegalBlock, QualityMetadata, ReadingChapter, ReadingSummary},
        output_contract::GenerationMode,
    };
    use astral_llm_infra::CanonicalCatalog;
    use std::sync::Arc;

    fn catalog_with_symbolic() -> SharedCanonicalCatalog {
        Arc::new(CanonicalCatalog {
            safety_patterns: vec![astral_llm_infra::SafetyPattern {
                pattern_type: "symbolic".into(),
                locale: "fr".into(),
                pattern: "symbolique".into(),
            }],
            ..Default::default()
        })
    }

    fn sample_reading(body: &str) -> NatalReadingResponse {
        NatalReadingResponse {
            schema_version: "natal_reading_v1".into(),
            language: "fr".into(),
            reading_type: "natal_basic".into(),
            summary: ReadingSummary {
                title: "T".into(),
                short_text: "S".into(),
            },
            chapters: vec![ReadingChapter {
                code: "identity".into(),
                title: "Identite".into(),
                body: body.into(),
                astro_basis: vec![],
                confidence: astral_llm_domain::ConfidenceLevel::Medium,
                safety_flags: vec![],
            }],
            legal: LegalBlock {
                disclaimer: "Disclaimer".into(),
            },
            quality: QualityMetadata {
                used_provider: "fake".into(),
                used_model: "fake".into(),
                generation_mode: GenerationMode::SinglePass,
                prompt_family: "natal_basic".into(),
                prompt_version: "v1".into(),
                astro_contract_version: "v13".into(),
                fallback_used: false,
            },
        }
    }

    #[test]
    fn requires_symbolic_framing_when_policy_demands_it() {
        let reading = sample_reading("Texte neutre sans cadrage.");
        let policy = SafetyPolicy::mandatory();
        let result =
            SafetyGuard::validate_response(&reading, &policy, &[], &catalog_with_symbolic());
        assert!(result.is_err());
    }
}
