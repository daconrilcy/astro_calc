use astral_llm_domain::{
    generation_request::AudienceLevel,
    generation_response::NatalReadingResponse,
    output_contract::GenerationMode,
    GenerateReadingRequest, GenerationError, GenerationErrorCode,
};

use crate::astro_label_humanizer::AstroLabelHumanizer;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::text_trigrams::count_repeated_trigrams;

#[derive(Debug, Clone, Default)]
pub struct ReadingQualityReport {
    pub chapter_length_ok: bool,
    pub interpretive_framing_ok: bool,
    pub repetition_ok: bool,
    pub deterministic_claims_ok: bool,
    pub disclaimer_ok: bool,
    pub astro_basis_density_ok: bool,
    pub warnings: Vec<String>,
}

impl ReadingQualityReport {
    pub fn is_acceptable(&self) -> bool {
        self.warnings.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct PremiumQualityThresholds {
    pub min_words_per_chapter: usize,
    pub max_repeated_trigrams: usize,
    pub min_astro_basis_per_chapter: u8,
}

impl Default for PremiumQualityThresholds {
    fn default() -> Self {
        Self {
            min_words_per_chapter: 40,
            max_repeated_trigrams: 3,
            min_astro_basis_per_chapter: 1,
        }
    }
}

pub struct ReadingQualityValidator;

/// Gate bloquante selon le profil d'interpretation (`natal_prompter` requiert un profil resolu).
pub fn requires_blocking_quality_gate(
    _request: &GenerateReadingRequest,
    interpretation: Option<&ResolvedInterpretationContext>,
) -> bool {
    interpretation
        .map(|ctx| ctx.profile.blocking_quality_gate())
        .unwrap_or(false)
}

impl ReadingQualityValidator {
    pub fn assess(
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
        interpretation: Option<&ResolvedInterpretationContext>,
    ) -> ReadingQualityReport {
        let thresholds = thresholds_for_request(request, interpretation);
        let locale = AstroLabelHumanizer::locale_key(&request.product_context.user_language);
        let blocking = requires_blocking_quality_gate(request, interpretation);
        Self::assess_with_thresholds(request, reading, &thresholds, locale, blocking)
    }

    pub fn assess_with_thresholds(
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
        thresholds: &PremiumQualityThresholds,
        locale: &str,
        blocking_gate: bool,
    ) -> ReadingQualityReport {
        let mut report = ReadingQualityReport::default();
        let mut warnings = Vec::new();
        let corpus = reading
            .chapters
            .iter()
            .map(|c| c.body.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");

        if blocking_gate && reading.chapters.is_empty() {
            warnings.push("premium reading has no chapters".into());
        }

        for chapter in &reading.chapters {
            let words = word_count(&chapter.body);
            if words < thresholds.min_words_per_chapter {
                warnings.push(format!(
                    "chapter '{}' too short ({words} words, min {})",
                    chapter.code, thresholds.min_words_per_chapter
                ));
            }

            if !has_interpretive_framing(&chapter.body) {
                warnings.push(format!(
                    "chapter '{}' lacks interpretive framing",
                    chapter.code
                ));
            }

            let repeats = count_repeated_trigrams(&chapter.body, locale);
            if repeats > thresholds.max_repeated_trigrams {
                warnings.push(format!(
                    "chapter '{}' repetition score too high ({repeats})",
                    chapter.code
                ));
            }

            let valid_basis = chapter
                .astro_basis
                .iter()
                .filter(|b| !b.factor.trim().is_empty())
                .count();
            if valid_basis < thresholds.min_astro_basis_per_chapter as usize {
                warnings.push(format!(
                    "chapter '{}' astro_basis density too low ({valid_basis})",
                    chapter.code
                ));
            }
        }

        if request.response_contract.include_legal_disclaimer && reading.legal.disclaimer.is_empty() {
            warnings.push("legal disclaimer missing".into());
        }

        if has_deterministic_wording(&corpus) {
            warnings.push("deterministic wording detected".into());
        }

        if matches!(request.product_context.audience_level, AudienceLevel::Beginner) {
            if has_beginner_jargon(&corpus) {
                warnings.push("beginner audience contains excessive jargon".into());
            }
        }

        report.chapter_length_ok = !warnings.iter().any(|w| w.contains("too short"));
        report.interpretive_framing_ok = !warnings.iter().any(|w| w.contains("interpretive"));
        report.repetition_ok = !warnings.iter().any(|w| w.contains("repetition"));
        report.deterministic_claims_ok = !warnings.iter().any(|w| w.contains("deterministic"));
        report.disclaimer_ok = !warnings.iter().any(|w| w.contains("disclaimer"));
        report.astro_basis_density_ok = !warnings.iter().any(|w| w.contains("astro_basis"));
        report.warnings = warnings;
        report
    }

    pub fn chapter_repetition_score(body: &str, locale: &str) -> usize {
        count_repeated_trigrams(body, locale)
    }

    pub fn chapter_exceeds_repetition(
        body: &str,
        thresholds: &PremiumQualityThresholds,
        locale: &str,
    ) -> bool {
        count_repeated_trigrams(body, locale) > thresholds.max_repeated_trigrams
    }

    /// Profils non bloquants : log warnings. Profils bloquants : echec si qualite insuffisante.
    pub fn validate_for_product(
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
        interpretation: Option<&ResolvedInterpretationContext>,
    ) -> Result<ReadingQualityReport, GenerationError> {
        let report = Self::assess(request, reading, interpretation);
        if requires_blocking_quality_gate(request, interpretation) {
            if !report.is_acceptable() {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::ReadingQualityFailed,
                    "premium reading quality below threshold",
                    serde_json::json!({
                        "warnings": report.warnings,
                        "product_code": request.product_context.product_code,
                        "generation_mode": request.response_contract.generation_mode.as_str(),
                    }),
                ));
            }
            return Ok(report);
        }

        if !report.is_acceptable() {
            tracing::warn!(
                warnings = ?report.warnings,
                "reading quality below expectations (non-blocking for basic)"
            );
        }
        Ok(report)
    }

    #[deprecated(note = "use validate_for_product")]
    pub fn assess_or_warn(
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
    ) -> ReadingQualityReport {
        Self::validate_for_product(request, reading, None)
            .unwrap_or_else(|_| Self::assess(request, reading, None))
    }
}

pub fn thresholds_for_request(
    request: &GenerateReadingRequest,
    interpretation: Option<&ResolvedInterpretationContext>,
) -> PremiumQualityThresholds {
    if let Some(ctx) = interpretation {
        let q = &ctx.profile.document.quality;
        let mut min_astro = q.min_astro_basis_refs_per_chapter;
        if let Some(policy) = ctx.profile.to_premium_evidence_policy() {
            min_astro = min_astro.max(policy.min_evidence_per_chapter);
        }
        return PremiumQualityThresholds {
            min_words_per_chapter: q.min_words_per_chapter as usize,
            max_repeated_trigrams: q.max_repeated_trigrams as usize,
            min_astro_basis_per_chapter: min_astro,
        };
    }
    let mut t = PremiumQualityThresholds::default();
    if matches!(
        request.response_contract.generation_mode,
        GenerationMode::SinglePass
    ) {
        t.min_astro_basis_per_chapter = 0;
    }
    t
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn has_interpretive_framing(body: &str) -> bool {
    let lower = body.to_lowercase();
    [
        "symbolique", "interpretation", "interprétation", "suggere", "suggère", "invite",
        "tendance", "peut", "offre", "révèle", "revel", "met en lumière", "met en lumiere",
        "suggests", "invites", "tendency", "may", "offers",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

#[cfg(test)]
mod repetition_tests {
    use crate::text_trigrams::count_repeated_trigrams;

    #[test]
    fn counts_distinct_repeated_phrases_not_every_window() {
        let body = "votre theme invite votre theme invite votre theme invite \
            a explorer la vie interieure avec douceur et clarte symbolique \
            pour comprendre les emotions et les liens humains avec bienveillance";
        let score = count_repeated_trigrams(body, "fr");
        assert!(score <= 3, "expected at most 3 distinct repeats, got {score}");
    }
}

fn has_deterministic_wording(corpus: &str) -> bool {
    [
        "destin inevitable",
        "sera inevitablement",
        "a coup sur",
        "sans aucun doute vous allez",
        "definitely will happen",
        "certain death",
    ]
    .iter()
    .any(|p| corpus.contains(p))
}

fn has_beginner_jargon(corpus: &str) -> bool {
    ["maison xii", "quincunx", "pars fortunae", "biquintile"]
        .iter()
        .any(|j| corpus.contains(j))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::{
        generation_response::{
            ConfidenceLevel, LegalBlock, NatalReadingResponse, QualityMetadata, ReadingChapter,
            ReadingSummary,
        },
        output_contract::GenerationMode,
    };

    fn premium_ctx() -> ResolvedInterpretationContext {
        let profile = astral_llm_infra::bootstrap_interpretation_profiles()
            .get("natal_premium")
            .expect("natal_premium profile")
            .clone();
        let effective_policy = profile.to_product_generation_policy();
        ResolvedInterpretationContext {
            profile,
            effective_policy,
        }
    }

    fn premium_request() -> GenerateReadingRequest {
        GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: astral_llm_domain::ProductContext {
                product_code: "natal_prompter".into(),
                interpretation_profile_code: Some("natal_premium".into()),
                user_language: "fr".into(),
                audience_level: AudienceLevel::Intermediate,
            },
            astro_result: astral_llm_domain::AstroCalculationPayload {
                contract_version: "natal_structured_v13".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({
                    "domain_scores": { "identity": 0.5 },
                    "planets": {
                        "sun": { "house": 2, "sign": "capricorn" }
                    }
                }),
            },
            astrologer_profile: astral_llm_domain::AstrologerProfile {
                profile_id: None,
                name: None,
                tone: astral_llm_domain::ToneProfile::Warm,
                jargon_level: astral_llm_domain::JargonLevel::Balanced,
                wording_style: astral_llm_domain::WordingStyle::Clear,
                preferred_domains: vec!["identity".into()],
                forbidden_wording: vec![],
                custom_instructions: None,
            },
            engine: astral_llm_domain::EngineParams {
                provider: None,
                model: None,
                reasoning_effort: None,
                temperature: None,
                max_output_tokens: None,
                domain_count: None,
                allow_fallback: false,
                timeout_ms: None,
                allow_oracle_benchmark: false,
                summary_model: None,
            },
            response_contract: astral_llm_domain::ResponseContract {
                output_schema_version: "natal_reading_v1".into(),
                generation_mode: GenerationMode::ChapterOrchestrated,
                format: astral_llm_domain::OutputFormat::StructuredJson,
                chapters: vec![],
                global_max_tokens: None,
                include_astro_sources: true,
                include_legal_disclaimer: true,
            },
            safety_policy: None,
        }
    }

    fn good_reading() -> NatalReadingResponse {
        NatalReadingResponse {
            schema_version: "natal_reading_v1".into(),
            language: "fr".into(),
            reading_type: "natal_prompter".into(),
            summary: ReadingSummary {
                title: "T".into(),
                short_text: "S".into(),
            },
            chapters: vec![ReadingChapter {
                code: "identity".into(),
                title: "Identite".into(),
                body: "Votre theme suggere une personnalite reflechie, orientee vers la \
                    comprehension symbolique des experiences et des transitions interieures. \
                    Vous avancez avec prudence lorsque le sens n'est pas clair, tout en montrant \
                    une grande capacite d'adaptation lorsque vous sentez une direction authentique. \
                    Cette configuration invite a accueillir les phases de questionnement comme \
                    des espaces creatifs, plutot que comme des blocages rigides.".into(),
                astro_basis: vec![
                    astral_llm_domain::AstroBasisItem {
                        fact_id: Some("domain_score:identity".into()),
                        label: None,
                        factor: "identity".into(),
                        interpretive_role: "domain_score".into(),
                    },
                    astral_llm_domain::AstroBasisItem {
                        fact_id: Some("placement:sun:capricorn:house:2".into()),
                        label: None,
                        factor: "sun".into(),
                        interpretive_role: "core".into(),
                    },
                    astral_llm_domain::AstroBasisItem {
                        fact_id: Some("placement:moon:cancer:house:4".into()),
                        label: None,
                        factor: "moon".into(),
                        interpretive_role: "core".into(),
                    },
                    astral_llm_domain::AstroBasisItem {
                        fact_id: Some("aspect:sun:moon:trine".into()),
                        label: None,
                        factor: "sun_moon".into(),
                        interpretive_role: "supporting".into(),
                    },
                ],
                confidence: ConfidenceLevel::Medium,
                safety_flags: vec![],
            }],
            legal: LegalBlock {
                disclaimer: "Lecture symbolique.".into(),
            },
            quality: QualityMetadata {
                used_provider: "fake".into(),
                used_model: "fake".into(),
                generation_mode: GenerationMode::ChapterOrchestrated,
                prompt_family: "natal_prompter".into(),
                prompt_version: "v1".into(),
                astro_contract_version: "natal_structured_v13".into(),
                fallback_used: false,
            },
        }
    }

    #[test]
    fn premium_rejects_poor_quality() {
        let request = premium_request();
        let mut reading = good_reading();
        reading.chapters[0].body = "sun in aries. moon in cancer.".into();
        let ctx = premium_ctx();
        assert!(
            ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_err()
        );
    }

    #[test]
    fn premium_accepts_rich_reading() {
        let request = premium_request();
        let reading = good_reading();
        let ctx = premium_ctx();
        assert!(
            ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_ok()
        );
    }

    #[test]
    fn chapter_orchestrated_without_profile_does_not_block() {
        let request = premium_request();
        assert!(!requires_blocking_quality_gate(&request, None));
    }

    #[test]
    fn premium_profile_blocks_even_in_single_pass_mode() {
        let mut request = premium_request();
        request.response_contract.generation_mode = GenerationMode::SinglePass;
        let mut reading = good_reading();
        reading.chapters[0].body = "sun aries. moon cancer.".into();
        let ctx = premium_ctx();
        assert!(
            ReadingQualityValidator::validate_for_product(&request, &reading, Some(&ctx)).is_err()
        );
    }
}
