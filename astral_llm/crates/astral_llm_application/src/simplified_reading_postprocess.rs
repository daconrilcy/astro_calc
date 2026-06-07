use astral_llm_domain::{
    default_legal_disclaimer,
    generation_response::{NatalReadingResponse, ReadingChapter, ReadingSummary},
    GenerateReadingRequest,
};

use crate::french_typography::restore_french_elisions;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::reading_script_guard::sanitize_text_for_french_script;
use astral_llm_domain::generation_response::ConfidenceLevel;

use crate::simplified_reading::{
    sun_sign_blocked, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_PROFILE,
};
use crate::summary_ux_rules::{count_words, split_sentences_fr, SummaryUxRules};

pub const SCRIPT_REPAIR_INSTRUCTION: &str =
    "Réécrivez entièrement en français avec l'alphabet latin \
    (accents français autorisés). Supprimez tout caractère d'un autre système d'écriture \
    (cyrillique, devanagari, arabe, etc.). Utilisez les apostrophes d'élision françaises \
    (l'identité, d'une, n'est, qu'elle, s'appuie). Ne changez pas le fond astrologique.";

const SIMPLIFIED_INTERPRETIVE_ROLES: &[&str] = &["core", "supporting", "nuance"];

#[derive(Debug, Clone, Default)]
pub struct AmbiguousCoreHardeningAudit {
    pub chapter_code_corrected: bool,
    pub confidence_clamped: bool,
    pub basis_pruned: usize,
    pub uncertainty_prefix_applied: bool,
}

impl AmbiguousCoreHardeningAudit {
    pub fn any_applied(&self) -> bool {
        self.chapter_code_corrected
            || self.confidence_clamped
            || self.basis_pruned > 0
            || self.uncertainty_prefix_applied
    }
}

#[derive(Debug, Clone, Default)]
pub struct SimplifiedPostProcessAudit {
    pub sanitized_fields: Vec<String>,
    pub typography_fields: Vec<String>,
    pub summary_source: Option<String>,
    pub body_fallback_applied: bool,
    pub interpretive_roles_normalized: usize,
    pub ambiguous_core_hardening: AmbiguousCoreHardeningAudit,
}

pub fn post_process_single_pass_reading(
    reading: &mut NatalReadingResponse,
    request: &GenerateReadingRequest,
    interpretation: Option<&ResolvedInterpretationContext>,
) -> SimplifiedPostProcessAudit {
    let mut audit = SimplifiedPostProcessAudit::default();
    let language = request.product_context.user_language.as_str();

    if request.response_contract.include_legal_disclaimer {
        reading.legal.disclaimer = default_legal_disclaimer(language, true);
    }

    let is_simplified = interpretation
        .map(|ctx| ctx.profile.profile_code == SIMPLIFIED_PROFILE)
        .unwrap_or(false);

    audit.sanitized_fields = sanitize_reading_text_fields(reading, language);
    audit.typography_fields = restore_french_typography_fields(reading, language);

    if is_simplified {
        audit.interpretive_roles_normalized = normalize_simplified_interpretive_roles(reading);
        let blocked = request
            .astro_result
            .data
            .get("llm_controls")
            .map(sun_sign_blocked)
            .unwrap_or(false);
        audit.ambiguous_core_hardening =
            harden_ambiguous_core_identity_chapter(reading, blocked, language);
        reading.summary = build_simplified_summary(reading, language);
        audit.summary_source = Some("server_compact_from_chapter".into());
    }

    audit
}

pub fn apply_simplified_body_fallback(reading: &mut NatalReadingResponse, chapter_code: &str) {
    let body = simplified_deterministic_body(chapter_code);
    if let Some(chapter) = reading.chapters.first_mut() {
        chapter.code = chapter_code.to_string();
        chapter.body = body;
        return;
    }
    reading.chapters.push(ReadingChapter {
        code: chapter_code.to_string(),
        title: if chapter_code == SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE {
            "Identité — Soleil ambigu".into()
        } else {
            "Identité".into()
        },
        body,
        astro_basis: vec![],
        confidence: ConfidenceLevel::Low,
        safety_flags: vec![],
    });
}

pub fn build_simplified_summary(reading: &NatalReadingResponse, language: &str) -> ReadingSummary {
    let chapter = reading.chapters.first();
    let title = chapter
        .map(|c| c.title.clone())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| simplified_summary_title(language));

    let short_text = chapter
        .map(|c| build_compact_summary_from_body(&c.body, language))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| simplified_summary_short_text(language));

    ReadingSummary { title, short_text }
}

pub fn build_compact_summary_from_body(body: &str, language: &str) -> String {
    let rules = SummaryUxRules::default();
    let sentences = split_sentences_fr(body);
    if sentences.is_empty() {
        return simplified_summary_short_text(language);
    }

    let mut picked = Vec::new();
    let mut words = 0usize;
    for sentence in &sentences {
        let sentence_words = count_words(sentence);
        if picked.len() >= rules.max_short_text_sentences {
            break;
        }
        if !picked.is_empty() && words + sentence_words > rules.max_short_text_words {
            break;
        }
        picked.push(sentence.clone());
        words += sentence_words;
    }

    if picked.is_empty() {
        let first = sentences.first().cloned().unwrap_or_default();
        if count_words(&first) <= rules.max_short_text_words {
            return first;
        }
        return trim_to_complete_sentence(&first, rules.max_short_text_words);
    }

    picked.join(" ")
}

fn trim_to_complete_sentence(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.trim().to_string();
    }
    let trimmed = words[..max_words].join(" ");
    if trimmed.ends_with(['.', '!', '?']) {
        trimmed
    } else {
        format!(
            "{}.",
            trimmed.trim_end_matches(|c: char| matches!(c, ',' | ';' | ':'))
        )
    }
}

fn simplified_summary_title(language: &str) -> String {
    if language.starts_with("fr") {
        "Lecture indicative".into()
    } else {
        "Indicative reading".into()
    }
}

pub fn simplified_summary_short_text(language: &str) -> String {
    if language.starts_with("fr") {
        "Interprétation astrologique partielle fondée sur les seules données de naissance fournies."
            .into()
    } else {
        "Partial astrological interpretation based only on the birth data provided.".into()
    }
}

pub fn simplified_deterministic_body(chapter_code: &str) -> String {
    if chapter_code == SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE {
        "Votre Soleil se situe dans une zone de changement possible entre deux signes. \
         Sans heure ou fuseau plus précis, on ne peut pas poser clairement le cœur solaire \
         du profil. Les autres placements stables peuvent néanmoins donner des indications \
         secondaires, avec prudence. Cette lecture reste indicative et ne remplace pas une \
         analyse complète du thème."
            .into()
    } else {
        "Cette lecture indicative repose sur les seules données de naissance disponibles. \
         Elle met en lumière des tendances symboliques plutôt qu'un portrait exhaustif. \
         Les éléments stables du thème peuvent néanmoins suggérer une personnalité réfléchie, \
         orientée vers la compréhension des expériences."
            .into()
    }
}

pub fn harden_ambiguous_core_identity_chapter(
    reading: &mut NatalReadingResponse,
    sun_sign_blocked: bool,
    language: &str,
) -> AmbiguousCoreHardeningAudit {
    let mut audit = AmbiguousCoreHardeningAudit::default();
    if !sun_sign_blocked {
        return audit;
    }
    let chapter_idx = reading
        .chapters
        .iter()
        .position(|ch| ch.code == SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE)
        .unwrap_or(0);
    let Some(chapter) = reading.chapters.get_mut(chapter_idx) else {
        return audit;
    };

    if chapter.code != SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE {
        chapter.code = SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE.into();
        audit.chapter_code_corrected = true;
    }
    if chapter.confidence != ConfidenceLevel::Low {
        chapter.confidence = ConfidenceLevel::Low;
        audit.confidence_clamped = true;
    }
    let before = chapter.astro_basis.len();
    chapter.astro_basis.retain(|basis| {
        !matches!(
            basis.fact_id.as_deref(),
            Some("placement:sun") | Some("placement:moon")
        )
    });
    audit.basis_pruned = before.saturating_sub(chapter.astro_basis.len());

    if language.trim().eq_ignore_ascii_case("fr")
        && !body_has_ambiguous_uncertainty_lexicon(&chapter.body)
    {
        let prefix = ambiguous_uncertainty_prefix_sentence();
        chapter.body = format!("{} {}", prefix, chapter.body.trim());
        audit.uncertainty_prefix_applied = true;
    }

    audit
}

pub fn body_has_ambiguous_uncertainty_lexicon(body: &str) -> bool {
    let lower = body.to_lowercase();
    ["soleil", "determin", "certitude", "changement", "zone"]
        .iter()
        .any(|token| lower.contains(token))
}

fn ambiguous_uncertainty_prefix_sentence() -> String {
    let body = simplified_deterministic_body(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
    split_sentences_fr(&body)
        .into_iter()
        .next()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            "Votre Soleil se situe dans une zone de changement possible entre deux signes.".into()
        })
}

pub fn normalize_simplified_interpretive_roles(reading: &mut NatalReadingResponse) -> usize {
    let mut normalized = 0usize;
    for chapter in &mut reading.chapters {
        for basis in &mut chapter.astro_basis {
            let role = basis.interpretive_role.trim().to_lowercase();
            if SIMPLIFIED_INTERPRETIVE_ROLES.contains(&role.as_str()) {
                continue;
            }
            basis.interpretive_role = "supporting".into();
            normalized += 1;
        }
    }
    normalized
}

fn sanitize_reading_text_fields(reading: &mut NatalReadingResponse, language: &str) -> Vec<String> {
    if !language.trim().eq_ignore_ascii_case("fr") {
        return Vec::new();
    }

    let mut sanitized = Vec::new();
    if sanitize_field("summary.title", &mut reading.summary.title, &mut sanitized) {}
    if sanitize_field(
        "summary.short_text",
        &mut reading.summary.short_text,
        &mut sanitized,
    ) {}
    for (i, chapter) in reading.chapters.iter_mut().enumerate() {
        sanitize_field(
            &format!("chapters[{i}].title"),
            &mut chapter.title,
            &mut sanitized,
        );
        sanitize_field(
            &format!("chapters[{i}].body"),
            &mut chapter.body,
            &mut sanitized,
        );
        for (j, basis) in chapter.astro_basis.iter_mut().enumerate() {
            if let Some(label) = basis.label.as_mut() {
                sanitize_field(
                    &format!("chapters[{i}].astro_basis[{j}].label"),
                    label,
                    &mut sanitized,
                );
            }
            sanitize_field(
                &format!("chapters[{i}].astro_basis[{j}].factor"),
                &mut basis.factor,
                &mut sanitized,
            );
        }
    }
    sanitized
}

fn restore_french_typography_fields(
    reading: &mut NatalReadingResponse,
    language: &str,
) -> Vec<String> {
    if !language.trim().eq_ignore_ascii_case("fr") {
        return Vec::new();
    }

    let mut restored = Vec::new();
    typography_field("summary.title", &mut reading.summary.title, &mut restored);
    typography_field(
        "summary.short_text",
        &mut reading.summary.short_text,
        &mut restored,
    );
    for (i, chapter) in reading.chapters.iter_mut().enumerate() {
        typography_field(
            &format!("chapters[{i}].title"),
            &mut chapter.title,
            &mut restored,
        );
        typography_field(
            &format!("chapters[{i}].body"),
            &mut chapter.body,
            &mut restored,
        );
        for (j, basis) in chapter.astro_basis.iter_mut().enumerate() {
            if let Some(label) = basis.label.as_mut() {
                typography_field(
                    &format!("chapters[{i}].astro_basis[{j}].label"),
                    label,
                    &mut restored,
                );
            }
            typography_field(
                &format!("chapters[{i}].astro_basis[{j}].factor"),
                &mut basis.factor,
                &mut restored,
            );
        }
    }
    restored
}

fn sanitize_field(field: &str, text: &mut String, out: &mut Vec<String>) -> bool {
    let (clean, changed) = sanitize_text_for_french_script(text);
    if changed {
        *text = clean;
        out.push(field.to_string());
    }
    changed
}

fn typography_field(field: &str, text: &mut String, out: &mut Vec<String>) {
    let (fixed, changed) = restore_french_elisions(text);
    if changed {
        *text = fixed;
        out.push(field.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::generation_response::{
        AstroBasisItem, ConfidenceLevel, LegalBlock, QualityMetadata, ReadingChapter,
        ReadingSummary,
    };
    use astral_llm_domain::output_contract::GenerationMode;

    fn sample_reading(body: &str) -> NatalReadingResponse {
        NatalReadingResponse {
            schema_version: "natal_reading_v1".into(),
            language: "fr".into(),
            reading_type: "natal_prompter".into(),
            summary: ReadingSummary {
                title: "Identité".into(),
                short_text: "Résumé".into(),
            },
            chapters: vec![ReadingChapter {
                code: SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE.into(),
                title: "Identité — Soleil ambigu".into(),
                body: body.into(),
                astro_basis: vec![],
                confidence: ConfidenceLevel::Medium,
                safety_flags: vec![],
            }],
            legal: LegalBlock {
                disclaimer: String::new(),
            },
            quality: QualityMetadata {
                used_provider: "fake".into(),
                used_model: "fake".into(),
                generation_mode: GenerationMode::SinglePass,
                prompt_family: "natal_prompter".into(),
                prompt_version: "v1".into(),
                astro_contract_version: "natal_simplified_structured_v1".into(),
                fallback_used: false,
            },
        }
    }

    #[test]
    fn sanitize_removes_devanagari_from_body() {
        let mut reading = sample_reading("Texte avec संकेत parasite.");
        let fields = sanitize_reading_text_fields(&mut reading, "fr");
        assert!(!fields.is_empty());
        assert!(!reading.chapters[0].body.contains('\u{0938}'));
    }

    #[test]
    fn typography_restores_elisions_in_body() {
        let mut reading = sample_reading(
            "Avec le Soleil ambigu, l impression générale reste prudente. Ce n est pas une certitude.",
        );
        let fields = restore_french_typography_fields(&mut reading, "fr");
        assert!(!fields.is_empty());
        assert!(reading.chapters[0].body.contains("l'impression"));
        assert!(reading.chapters[0].body.contains("n'est"));
    }

    #[test]
    fn compact_summary_uses_complete_sentences_without_ellipsis() {
        let body = "Première phrase complète sur l'identité. Deuxième phrase qui nuance le portrait. \
                    Troisième phrase beaucoup plus longue qui ne devrait pas apparaître entièrement.";
        let summary = build_compact_summary_from_body(body, "fr");
        assert!(!summary.contains('…'));
        assert!(summary.starts_with("Première phrase complète"));
        assert!(summary.contains("Deuxième phrase"));
        assert!(!summary.contains("Troisième phrase"));
    }

    #[test]
    fn normalize_maps_domain_score_to_supporting() {
        let mut reading = sample_reading("Corps.");
        reading.chapters[0].astro_basis = vec![AstroBasisItem {
            fact_id: Some("placement:saturn".into()),
            label: Some("Saturne".into()),
            factor: "Saturne en Capricorne".into(),
            interpretive_role: "domain_score".into(),
        }];
        assert_eq!(normalize_simplified_interpretive_roles(&mut reading), 1);
        assert_eq!(
            reading.chapters[0].astro_basis[0].interpretive_role,
            "supporting"
        );
    }

    #[test]
    fn ambiguous_body_fallback_is_french_only() {
        let body = simplified_deterministic_body(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
        assert!(body.contains("zone de changement"));
        assert!(crate::reading_script_guard::script_violations_for_reading(
            "fr",
            &sample_reading(&body)
        )
        .is_empty());
    }

    #[test]
    fn harden_clamps_high_confidence_to_low() {
        let mut reading = sample_reading("Corps sans lexique.");
        reading.chapters[0].confidence = ConfidenceLevel::High;
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert!(audit.confidence_clamped);
        assert_eq!(reading.chapters[0].confidence, ConfidenceLevel::Low);
    }

    #[test]
    fn harden_prunes_sun_and_moon_basis() {
        let mut reading = sample_reading("Soleil ambigu et zone de changement.");
        reading.chapters[0].astro_basis = vec![
            AstroBasisItem {
                fact_id: Some("placement:sun".into()),
                label: None,
                factor: "Soleil".into(),
                interpretive_role: "core".into(),
            },
            AstroBasisItem {
                fact_id: Some("placement:mercury".into()),
                label: None,
                factor: "Mercure".into(),
                interpretive_role: "supporting".into(),
            },
        ];
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert_eq!(audit.basis_pruned, 1);
        assert_eq!(reading.chapters[0].astro_basis.len(), 1);
        assert_eq!(
            reading.chapters[0].astro_basis[0].fact_id.as_deref(),
            Some("placement:mercury")
        );
    }

    #[test]
    fn harden_prefixes_body_without_uncertainty_lexicon() {
        let mut reading = sample_reading("Portrait general sans reference astrologique explicite.");
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert!(audit.uncertainty_prefix_applied);
        assert!(reading.chapters[0].body.contains("zone de changement"));
        assert!(body_has_ambiguous_uncertainty_lexicon(
            &reading.chapters[0].body
        ));
    }

    #[test]
    fn harden_prefix_is_idempotent_when_lexicon_present() {
        let mut reading =
            sample_reading("Le soleil reste incertain dans une zone de changement sans certitude.");
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert!(!audit.uncertainty_prefix_applied);
    }

    #[test]
    fn harden_corrects_chapter_code_when_sun_blocked() {
        let mut reading = sample_reading("Soleil ambigu dans une zone de changement.");
        reading.chapters[0].code = "identity".into();
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert!(audit.chapter_code_corrected);
        assert_eq!(reading.chapters[0].code, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
    }

    #[test]
    fn harden_skips_stable_identity_case() {
        let mut reading = sample_reading("Identite stable.");
        reading.chapters[0].code = "identity".into();
        reading.chapters[0].confidence = ConfidenceLevel::High;
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, false, "fr");
        assert!(!audit.any_applied());
        assert_eq!(reading.chapters[0].code, "identity");
        assert_eq!(reading.chapters[0].confidence, ConfidenceLevel::High);
    }

    #[test]
    fn harden_targets_ambiguous_chapter_not_only_first_index() {
        let mut reading = NatalReadingResponse {
            schema_version: "natal_reading_v1".into(),
            language: "fr".into(),
            reading_type: "natal_prompter".into(),
            summary: ReadingSummary {
                title: "T".into(),
                short_text: "S".into(),
            },
            chapters: vec![
                ReadingChapter {
                    code: "identity".into(),
                    title: "Brouillon".into(),
                    body: "Brouillon.".into(),
                    astro_basis: vec![],
                    confidence: ConfidenceLevel::Medium,
                    safety_flags: vec![],
                },
                ReadingChapter {
                    code: SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE.into(),
                    title: "Ambigu".into(),
                    body: "Texte sans lexique d incertitude explicite.".into(),
                    astro_basis: vec![],
                    confidence: ConfidenceLevel::High,
                    safety_flags: vec![],
                },
            ],
            legal: LegalBlock {
                disclaimer: String::new(),
            },
            quality: QualityMetadata {
                used_provider: "fake".into(),
                used_model: "fake".into(),
                generation_mode: GenerationMode::SinglePass,
                prompt_family: "natal_prompter".into(),
                prompt_version: "v1".into(),
                astro_contract_version: "natal_simplified_structured_v1".into(),
                fallback_used: false,
            },
        };
        let audit = harden_ambiguous_core_identity_chapter(&mut reading, true, "fr");
        assert!(audit.confidence_clamped);
        assert_eq!(reading.chapters[0].code, "identity");
        assert_eq!(reading.chapters[1].confidence, ConfidenceLevel::Low);
        assert!(body_has_ambiguous_uncertainty_lexicon(
            &reading.chapters[1].body
        ));
    }

    #[test]
    fn body_fallback_creates_chapter_when_missing() {
        let mut reading = sample_reading("");
        reading.chapters.clear();
        apply_simplified_body_fallback(&mut reading, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
        assert_eq!(reading.chapters.len(), 1);
        assert_eq!(reading.chapters[0].code, SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
        assert!(reading.chapters[0].body.contains("zone de changement"));
    }
}
