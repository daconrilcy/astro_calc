use astral_llm_domain::{
    default_legal_disclaimer,
    generation_response::{NatalReadingResponse, ReadingSummary},
    GenerateReadingRequest,
};

use crate::interpretation_profile_resolver::ResolvedInterpretationContext;
use crate::reading_script_guard::sanitize_text_for_french_script;
use crate::simplified_reading::{SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_PROFILE};

pub const SCRIPT_REPAIR_INSTRUCTION: &str = "Réécrivez entièrement en français avec l'alphabet latin \
    (accents français autorisés). Supprimez tout caractère d'un autre système d'écriture \
    (cyrillique, devanagari, arabe, etc.). Ne changez pas le fond astrologique.";

#[derive(Debug, Clone, Default)]
pub struct SimplifiedPostProcessAudit {
    pub sanitized_fields: Vec<String>,
    pub summary_source: Option<String>,
    pub body_fallback_applied: bool,
}

pub fn post_process_single_pass_reading(
    reading: &mut NatalReadingResponse,
    request: &GenerateReadingRequest,
    interpretation: Option<&ResolvedInterpretationContext>,
) -> SimplifiedPostProcessAudit {
    let mut audit = SimplifiedPostProcessAudit::default();
    let language = request.product_context.user_language.as_str();

    if request.response_contract.include_legal_disclaimer {
        reading.legal.disclaimer =
            default_legal_disclaimer(language, true);
    }

    let is_simplified = interpretation
        .map(|ctx| ctx.profile.profile_code == SIMPLIFIED_PROFILE)
        .unwrap_or(false);

    if is_simplified {
        reading.summary = build_simplified_summary(reading, language);
        audit.summary_source = Some("server_from_chapter".into());
    }

    audit.sanitized_fields = sanitize_reading_text_fields(reading, language);
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
        .map(|c| truncate_words(&c.body, 45))
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| simplified_summary_short_text(language));

    ReadingSummary { title, short_text }
}

fn simplified_summary_title(language: &str) -> String {
    if language.starts_with("fr") {
        "Lecture indicative".into()
    } else {
        "Indicative reading".into()
    }
}

fn simplified_summary_short_text(language: &str) -> String {
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
    }
    sanitized
}

fn sanitize_field(field: &str, text: &mut String, out: &mut Vec<String>) -> bool {
    let (clean, changed) = sanitize_text_for_french_script(text);
    if changed {
        *text = clean;
        out.push(field.to_string());
    }
    changed
}

fn truncate_words(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        return text.trim().to_string();
    }
    format!("{}…", words[..max_words].join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::generation_response::{
        ConfidenceLevel, LegalBlock, QualityMetadata, ReadingChapter,
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
    fn ambiguous_body_fallback_is_french_only() {
        let body = simplified_deterministic_body(SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);
        assert!(body.contains("zone de changement"));
        assert!(crate::reading_script_guard::script_violations_for_reading("fr", &sample_reading(&body)).is_empty());
    }
}
