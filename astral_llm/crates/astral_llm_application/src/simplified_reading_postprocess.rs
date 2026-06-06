use astral_llm_domain::{
    default_legal_disclaimer,
    generation_response::{NatalReadingResponse, ReadingSummary},
    GenerateReadingRequest,
};

use crate::french_typography::restore_french_elisions;
use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::reading_script_guard::sanitize_text_for_french_script;
use crate::simplified_reading::{SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_PROFILE};
use crate::summary_ux_rules::{count_words, split_sentences_fr, SummaryUxRules};

pub const SCRIPT_REPAIR_INSTRUCTION: &str = "Réécrivez entièrement en français avec l'alphabet latin \
    (accents français autorisés). Supprimez tout caractère d'un autre système d'écriture \
    (cyrillique, devanagari, arabe, etc.). Utilisez les apostrophes d'élision françaises \
    (l'identité, d'une, n'est, qu'elle, s'appuie). Ne changez pas le fond astrologique.";

const SIMPLIFIED_INTERPRETIVE_ROLES: &[&str] = &["core", "supporting", "nuance"];

#[derive(Debug, Clone, Default)]
pub struct SimplifiedPostProcessAudit {
    pub sanitized_fields: Vec<String>,
    pub typography_fields: Vec<String>,
    pub summary_source: Option<String>,
    pub body_fallback_applied: bool,
    pub interpretive_roles_normalized: usize,
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
        reading.summary = build_simplified_summary(reading, language);
        audit.summary_source = Some("server_compact_from_chapter".into());
    }

    audit
}

pub fn apply_simplified_body_fallback(
    reading: &mut NatalReadingResponse,
    chapter_code: &str,
) {
    if let Some(chapter) = reading.chapters.first_mut() {
        chapter.body = simplified_deterministic_body(chapter_code);
    }
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
        assert!(
            crate::reading_script_guard::script_violations_for_reading("fr", &sample_reading(&body))
                .is_empty()
        );
    }
}
