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

        if policy.require_symbolic_framing && !has_symbolic_framing(&corpus, catalog) {
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

        violations.extend(crate::reading_script_guard::script_violations_for_reading(
            &reading.language,
            reading,
        ));

        if policy.require_disclaimer && reading.legal.disclaimer.trim().is_empty() {
            violations.push("missing legal disclaimer".into());
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    pub fn validate_chapter_text(
        body: &str,
        policy: &SafetyPolicy,
        forbidden_wording: &[String],
        catalog: &SharedCanonicalCatalog,
    ) -> Result<(), Vec<String>> {
        let mut violations = Vec::new();

        if policy.forbid_medical_advice
            && matches_patterns(body, &catalog.patterns_for_type("medical"))
        {
            violations.push("medical advice detected".into());
        }
        if policy.forbid_legal_advice && matches_patterns(body, &catalog.patterns_for_type("legal"))
        {
            violations.push("legal advice detected".into());
        }
        if policy.forbid_financial_advice
            && matches_patterns(body, &catalog.patterns_for_type("financial"))
        {
            violations.push("financial advice detected".into());
        }
        if policy.forbid_death_prediction
            && matches_patterns(body, &catalog.patterns_for_type("death"))
        {
            violations.push("death prediction detected".into());
        }
        if policy.forbid_pregnancy_prediction
            && matches_patterns(body, &catalog.patterns_for_type("pregnancy"))
        {
            violations.push("pregnancy prediction detected".into());
        }
        if policy.forbid_deterministic_claims
            && matches_patterns(body, &catalog.patterns_for_type("deterministic"))
        {
            violations.push("deterministic claim detected".into());
        }
        if policy.require_symbolic_framing && !has_symbolic_framing(body, catalog) {
            violations.push("missing symbolic/interpretive framing".into());
        }

        for topic in &policy.custom_forbidden_topics {
            if body.to_lowercase().contains(&topic.to_lowercase()) {
                violations.push(format!("forbidden topic detected: {topic}"));
            }
        }
        for word in forbidden_wording {
            if body.to_lowercase().contains(&word.to_lowercase()) {
                violations.push(format!("forbidden wording detected: {word}"));
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

pub fn ensure_symbolic_framing_text(
    text: &str,
    language: &str,
    catalog: &SharedCanonicalCatalog,
) -> String {
    if has_symbolic_framing(text, catalog) {
        return text.to_string();
    }

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return symbolic_framing_sentence(language).to_string();
    }

    format!("{trimmed} {}", symbolic_framing_sentence(language))
}

fn collect_text(reading: &NatalReadingResponse) -> String {
    let mut parts = vec![
        reading.summary.title.clone(),
        reading.summary.short_text.clone(),
        reading.legal.disclaimer.clone(),
    ];
    for chapter in &reading.chapters {
        parts.push(chapter.title.clone());
        parts.push(chapter.summary_sentence.clone());
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

fn has_symbolic_framing(text: &str, catalog: &SharedCanonicalCatalog) -> bool {
    if matches_patterns(text, &catalog.patterns_for_type("symbolic")) {
        return true;
    }
    has_builtin_interpretive_framing(text)
}

fn has_builtin_interpretive_framing(body: &str) -> bool {
    let lower = body.to_lowercase();
    [
        "symbolique",
        "interpretation",
        "interprétation",
        "suggere",
        "suggère",
        "invite",
        "tendance",
        "peut",
        "offre",
        "révèle",
        "revel",
        "met en lumière",
        "met en lumiere",
        "suggests",
        "invites",
        "tendency",
        "may",
        "offers",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn symbolic_framing_sentence(language: &str) -> &'static str {
    match language {
        "fr" => {
            "Sur le plan symbolique, cela suggere des tendances a relire a la lumiere de votre experience."
        }
        _ => "Symbolically, this suggests tendencies to read alongside lived experience.",
    }
}
