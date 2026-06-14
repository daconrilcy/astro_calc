//! Garde-fous specifiques au profil natal_simplified (whitelist astro_basis, signes bloques).

use std::collections::HashSet;

use astral_llm_domain::{
    generation_response::{NatalReadingResponse, ReadingChapter},
    GenerationError, GenerationErrorCode,
};
use astral_llm_infra::SharedCanonicalCatalog;

use crate::simplified_reading_postprocess::body_has_ambiguous_uncertainty_lexicon;

use crate::simplified_reading::{SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE, SIMPLIFIED_PROFILE};

const LUMINARY_FR: &[(&str, &str)] = &[
    ("sun", "soleil"),
    ("moon", "lune"),
    ("mercury", "mercure"),
    ("venus", "venus"),
    ("mars", "mars"),
    ("jupiter", "jupiter"),
    ("saturn", "saturne"),
    ("uranus", "uranus"),
    ("neptune", "neptune"),
    ("pluto", "pluton"),
];

pub fn is_simplified_profile(profile_code: Option<&str>) -> bool {
    profile_code == Some(SIMPLIFIED_PROFILE)
}

pub fn validate_allowed_astro_basis_ids(
    chapters: &[ReadingChapter],
    allowed_ids: &[String],
) -> Result<(), GenerationError> {
    let allowed: HashSet<&str> = allowed_ids.iter().map(String::as_str).collect();
    for chapter in chapters {
        for basis in &chapter.astro_basis {
            let Some(fact_id) = basis.fact_id.as_ref() else {
                continue;
            };
            if fact_id.starts_with("domain_score:") {
                continue;
            }
            if !allowed.contains(fact_id.as_str()) {
                return Err(GenerationError::with_details(
                    GenerationErrorCode::AstroBasisInvalid,
                    format!(
                        "chapter '{}' cites fact_id outside allowed_astro_basis_fact_ids: {fact_id}",
                        chapter.code
                    ),
                    serde_json::json!({
                        "chapter": chapter.code,
                        "fact_id": fact_id,
                        "allowed_astro_basis_fact_ids": allowed_ids,
                    }),
                ));
            }
        }
    }
    Ok(())
}

pub fn blocked_sign_affirmation_violations(
    reading: &NatalReadingResponse,
    blocked_codes: &[String],
    catalog: &SharedCanonicalCatalog,
    language: &str,
) -> Vec<String> {
    if blocked_codes.is_empty() {
        return Vec::new();
    }
    let lang = language.trim().to_lowercase();
    if lang != "fr" {
        return Vec::new();
    }

    let sign_labels = french_zodiac_labels(catalog);
    if sign_labels.is_empty() {
        return Vec::new();
    }

    let corpus = collect_reading_corpus(reading);
    let corpus_lower = corpus.to_lowercase();
    let mut violations = Vec::new();

    for code in blocked_codes {
        let Some(object_code) = code.strip_suffix(".sign") else {
            continue;
        };
        let Some(body_name) = french_body_name(object_code) else {
            continue;
        };
        if affirms_sign_for_body(&corpus_lower, body_name, &sign_labels) {
            violations.push(format!(
                "blocked interpretive affirmation for {code} (language={lang})"
            ));
        }
    }
    violations
}

pub fn profile_excluded_affirmation_violations(
    reading: &NatalReadingResponse,
    profile_excluded: &[String],
) -> Vec<String> {
    let corpus = collect_reading_corpus(reading).to_lowercase();
    let mut violations = Vec::new();

    if profile_excluded.iter().any(|c| c == "ascendant") {
        if affirms_ascendant_by_sign(&corpus) {
            violations
                .push("affirms ascendant by zodiac sign while profile excludes ascendant".into());
        }
    }
    if profile_excluded
        .iter()
        .any(|c| c == "houses" || c == "house_placements")
    {
        if affirms_house_placement(&corpus) {
            violations.push("affirms house placement while profile excludes houses".into());
        }
    }
    violations
}

pub fn ambiguous_core_identity_violations(
    reading: &NatalReadingResponse,
    sun_sign_blocked: bool,
    language: &str,
) -> Vec<String> {
    if !sun_sign_blocked {
        return Vec::new();
    }

    let mut violations = Vec::new();
    let ambiguous = reading
        .chapters
        .iter()
        .find(|ch| ch.code == SIMPLIFIED_CHAPTER_AMBIGUOUS_CORE);

    let Some(chapter) = ambiguous else {
        violations.push("ambiguous_core_identity chapter required when sun.sign blocked".into());
        return violations;
    };

    if chapter.confidence != astral_llm_domain::generation_response::ConfidenceLevel::Low {
        violations.push(format!(
            "ambiguous_core_identity confidence must be low (got {:?})",
            chapter.confidence
        ));
    }

    for basis in &chapter.astro_basis {
        if matches!(
            basis.fact_id.as_deref(),
            Some("placement:sun") | Some("placement:moon")
        ) {
            violations.push(format!(
                "ambiguous_core_identity forbidden basis: {}",
                basis.fact_id.as_deref().unwrap_or("?")
            ));
        }
    }

    if language.trim().eq_ignore_ascii_case("fr")
        && !body_has_ambiguous_uncertainty_lexicon(&chapter.body)
    {
        violations.push("ambiguous_core_identity missing uncertainty wording (fr)".into());
    }

    violations
}

pub fn violations_are_ambiguous_core_only(violations: &[String]) -> bool {
    !violations.is_empty()
        && violations
            .iter()
            .all(|v| v.starts_with("ambiguous_core_identity"))
}

fn collect_reading_corpus(reading: &NatalReadingResponse) -> String {
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

fn french_body_name(object_code: &str) -> Option<&'static str> {
    LUMINARY_FR
        .iter()
        .find(|(code, _)| *code == object_code)
        .map(|(_, fr)| *fr)
}

fn french_zodiac_labels(catalog: &SharedCanonicalCatalog) -> Vec<String> {
    let codes = [
        "aries",
        "taurus",
        "gemini",
        "cancer",
        "leo",
        "virgo",
        "libra",
        "scorpio",
        "sagittarius",
        "capricorn",
        "aquarius",
        "pisces",
    ];
    codes
        .iter()
        .filter_map(|code| catalog.sign_label("fr", code))
        .map(|s| s.to_lowercase())
        .collect()
}

fn affirms_sign_for_body(corpus_lower: &str, body_fr: &str, sign_labels: &[String]) -> bool {
    for sign in sign_labels {
        if corpus_lower.contains(&format!("{body_fr} en {sign}"))
            || corpus_lower.contains(&format!("{body_fr} est en {sign}"))
            || corpus_lower.contains(&format!("{body_fr} est {sign}"))
            || corpus_lower.contains(&format!("{body_fr} en signe {sign}"))
        {
            return true;
        }
    }
    false
}

fn affirms_ascendant_by_sign(corpus_lower: &str) -> bool {
    const SIGNS: &[&str] = &[
        "bélier",
        "taureau",
        "gémeaux",
        "cancer",
        "lion",
        "vierge",
        "balance",
        "scorpion",
        "sagittaire",
        "capricorne",
        "verseau",
        "poissons",
    ];
    for sign in SIGNS {
        if corpus_lower.contains(&format!("ascendant en {sign}"))
            || corpus_lower.contains(&format!("ascendant est en {sign}"))
            || corpus_lower.contains(&format!("ascendant est {sign}"))
            || corpus_lower.contains(&format!("ascendant du {sign}"))
            || corpus_lower.contains(&format!("ascendant de {sign}"))
        {
            return true;
        }
    }
    false
}

fn affirms_house_placement(corpus_lower: &str) -> bool {
    for n in 1..=12 {
        if corpus_lower.contains(&format!(" en maison {n}"))
            || corpus_lower.contains(&format!(" en maison {n},"))
            || corpus_lower.contains(&format!(" en maison {n}."))
            || corpus_lower.contains(&format!(" maison {n} "))
        {
            return true;
        }
    }
    false
}
