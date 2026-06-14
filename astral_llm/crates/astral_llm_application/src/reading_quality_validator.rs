use astral_llm_domain::{
    generation_request::AudienceLevel, generation_response::NatalReadingResponse,
    interpretation_profile::SYNTHESIS_CHAPTER_CODE, output_contract::GenerationMode,
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
        let synthesis_min_astro =
            interpretation.map(|c| c.profile.synthesis_min_astro_basis_refs());
        let synthesis_min_words =
            interpretation.map(|c| c.profile.synthesis_word_targets().0 as usize);
        Self::assess_with_thresholds(
            request,
            reading,
            &thresholds,
            locale,
            blocking,
            synthesis_min_astro,
            synthesis_min_words,
        )
    }

    pub fn assess_with_thresholds(
        request: &GenerateReadingRequest,
        reading: &NatalReadingResponse,
        thresholds: &PremiumQualityThresholds,
        locale: &str,
        blocking_gate: bool,
        synthesis_min_astro_basis: Option<u8>,
        synthesis_min_words: Option<usize>,
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
            let min_words = if chapter.code == SYNTHESIS_CHAPTER_CODE {
                synthesis_min_words.unwrap_or(thresholds.min_words_per_chapter)
            } else {
                thresholds.min_words_per_chapter
            };
            if words < min_words {
                warnings.push(format!(
                    "chapter '{}' too short ({words} words, min {min_words})",
                    chapter.code
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
            let min_basis = if chapter.code == SYNTHESIS_CHAPTER_CODE {
                synthesis_min_astro_basis.unwrap_or(thresholds.min_astro_basis_per_chapter) as usize
            } else {
                thresholds.min_astro_basis_per_chapter as usize
            };
            if valid_basis < min_basis {
                warnings.push(format!(
                    "chapter '{}' astro_basis density too low ({valid_basis})",
                    chapter.code
                ));
            }
        }

        if request.response_contract.include_legal_disclaimer && reading.legal.disclaimer.is_empty()
        {
            warnings.push("legal disclaimer missing".into());
        }

        if has_deterministic_wording(&corpus) {
            warnings.push("deterministic wording detected".into());
        }

        let symbolic_boilerplate_chapters =
            count_symbolic_disclaimer_boilerplate_chapters(&reading.chapters);
        if symbolic_boilerplate_chapters > 2 {
            warnings.push(format!(
                "symbolic disclaimer boilerplate repeated in {symbolic_boilerplate_chapters} domain chapters (max 2)"
            ));
        }

        if matches!(
            request.product_context.audience_level,
            AudienceLevel::Beginner
        ) {
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
    .any(|pattern| corpus.contains(pattern))
}

fn has_beginner_jargon(corpus: &str) -> bool {
    ["maison xii", "quincunx", "pars fortunae", "biquintile"]
        .iter()
        .any(|jargon| corpus.contains(jargon))
}

const SYMBOLIC_DISCLAIMER_STOCK_PHRASES: &[&str] = &[
    "lecture reste symbolique",
    "lecture astrologique reste symbolique",
    "dans une lecture symbolique",
    "cette lecture reste symbolique",
];

fn chapter_has_symbolic_disclaimer_boilerplate(body: &str) -> bool {
    let lower = body.to_lowercase();
    SYMBOLIC_DISCLAIMER_STOCK_PHRASES
        .iter()
        .any(|phrase| lower.contains(phrase))
}

fn count_symbolic_disclaimer_boilerplate_chapters(
    chapters: &[astral_llm_domain::generation_response::ReadingChapter],
) -> usize {
    chapters
        .iter()
        .filter(|chapter| chapter.code != SYNTHESIS_CHAPTER_CODE)
        .filter(|chapter| chapter_has_symbolic_disclaimer_boilerplate(&chapter.body))
        .count()
}
