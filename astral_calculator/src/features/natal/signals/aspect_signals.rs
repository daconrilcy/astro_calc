//! Module astral_calculator\src\features\natal\signals\aspect_signals.rs du moteur astral_calculator.

use serde_json::json;

use crate::domain::AspectFact;

use super::tags::dedupe_tags;

pub(super) fn aspect_semantic_tags(
    aspect: &AspectFact,
    strength_score: f64,
    aspect_min_strength: f64,
) -> Vec<String> {
    let mut tags = vec![
        "aspect".to_string(),
        aspect.aspect_code.clone(),
        aspect.aspect_family.clone(),
        aspect_dynamic_quality(aspect).to_string(),
    ];
    if let Some(primary_valence) = aspect.primary_valence.as_ref() {
        tags.push(primary_valence.clone());
    }
    if let Some(intensity_modifier) = aspect.intensity_modifier.as_ref() {
        tags.push(intensity_modifier.clone());
    }
    if let Some(secondary_effect) = aspect.secondary_effect.as_ref() {
        tags.push(secondary_effect.clone());
    }
    if strength_score >= 0.75 {
        tags.push("high_strength".to_string());
    } else if strength_score < aspect_min_strength {
        tags.push("low_strength".to_string());
    }
    dedupe_tags(tags)
}

pub(super) fn aspect_context(aspect: &AspectFact) -> serde_json::Value {
    json!({
        "aspect_family": aspect.aspect_family,
        "primary_valence": aspect.primary_valence,
        "intensity_modifier": aspect.intensity_modifier,
        "secondary_effect": aspect.secondary_effect,
        "dynamic_quality": aspect_dynamic_quality(aspect),
        "phase_state": aspect.phase_state,
        "valence_family": aspect.valence_family,
        "is_tonal_valence": aspect.valence_is_tonal,
        "is_intensity_modifier": aspect.valence_is_intensity_modifier
    })
}

/// Fonction aspect_dynamic_quality.
fn aspect_dynamic_quality(aspect: &AspectFact) -> &'static str {
    match aspect.primary_valence.as_deref() {
        Some(
            "supportive" | "harmonious" | "creative" | "refined_creative" | "creative_ordering",
        ) => "flow",
        Some("dynamic_challenging" | "polarizing" | "minor_friction" | "indirect_tension") => {
            "tension"
        }
        Some("adjustment" | "subtle_adjustment") => "adjustment",
        Some("symbolic_fated") => "symbolic",
        Some("spiritual_integration") => "integration",
        Some(_) => "contextual",
        None if aspect.intensity_modifier.is_some() => "intensification",
        None => "contextual",
    }
}

pub(super) fn aspect_interpretive_hint(
    aspect: &AspectFact,
    aspect_name: &str,
    locale: &str,
) -> String {
    match locale {
        "fr" => format!(
            "Lisez ce {aspect_name} comme {} entre {} et {}, en prêtant attention à la phase {}.",
            aspect_hint_quality_phrase(aspect, locale),
            aspect.source_object_name,
            aspect.target_object_name,
            aspect.phase_state
        ),
        "es" => format!(
            "Lea este {aspect_name} como {} entre {} y {}, prestando atención a la fase {}.",
            aspect_hint_quality_phrase(aspect, locale),
            aspect.source_object_name,
            aspect.target_object_name,
            aspect.phase_state
        ),
        "de" => format!(
            "Lesen Sie diesen {aspect_name} als {} zwischen {} und {}, mit Blick auf die {}-Phase.",
            aspect_hint_quality_phrase(aspect, locale),
            aspect.source_object_name,
            aspect.target_object_name,
            aspect.phase_state
        ),
        _ => format!(
            "Read this {aspect_name} as {} between {} and {}, with attention to the {} phase.",
            aspect_hint_quality_phrase(aspect, locale),
            aspect.source_object_name,
            aspect.target_object_name,
            aspect.phase_state
        ),
    }
}

/// Fonction aspect_hint_quality_phrase.
fn aspect_hint_quality_phrase(aspect: &AspectFact, locale: &str) -> String {
    let base = match aspect.primary_valence.as_deref() {
        Some("supportive") => match locale {
            "fr" => "un flux soutenant",
            "es" => "un flujo de apoyo",
            "de" => "ein unterstützender Fluss",
            _ => "a supportive flow",
        },
        Some("harmonious") => match locale {
            "fr" => "un flux naturel",
            "es" => "un flujo armónico",
            "de" => "ein harmonischer Fluss",
            _ => "a natural flow",
        },
        Some("creative" | "refined_creative" | "creative_ordering") => match locale {
            "fr" => "une ouverture créative",
            "es" => "una apertura creativa",
            "de" => "eine kreative Öffnung",
            _ => "a creative opening",
        },
        Some("dynamic_challenging") => match locale {
            "fr" => "une tension active",
            "es" => "una tensión activa",
            "de" => "eine aktive Spannung",
            _ => "an active tension",
        },
        Some("polarizing") => match locale {
            "fr" => "une polarité à équilibrer",
            "es" => "una polaridad a equilibrar",
            "de" => "eine auszubalancierende Polarität",
            _ => "a polarity to balance",
        },
        Some("minor_friction") => match locale {
            "fr" => "une friction gérable",
            "es" => "una fricción manejable",
            "de" => "eine handhabbare Reibung",
            _ => "manageable friction",
        },
        Some("indirect_tension") => match locale {
            "fr" => "une tension indirecte",
            "es" => "una tensión indirecta",
            "de" => "eine indirekte Spannung",
            _ => "indirect tension",
        },
        Some("adjustment") => match locale {
            "fr" => "un ajustement",
            "es" => "un ajuste",
            "de" => "eine Anpassung",
            _ => "an adjustment",
        },
        Some("subtle_adjustment") => match locale {
            "fr" => "un ajustement subtil",
            "es" => "un ajuste sutil",
            "de" => "eine subtile Anpassung",
            _ => "a subtle adjustment",
        },
        Some("symbolic_fated") => match locale {
            "fr" => "une mise en relief symbolique",
            "es" => "un énfasis simbólico",
            "de" => "eine symbolische Betonung",
            _ => "a symbolic emphasis",
        },
        Some("spiritual_integration") => match locale {
            "fr" => "un lien intégrateur",
            "es" => "un vínculo integrador",
            "de" => "eine integrierende Verbindung",
            _ => "an integrating connection",
        },
        Some(_) => match locale {
            "fr" => "une relation contextuelle",
            "es" => "una relación contextual",
            "de" => "eine kontextuelle Beziehung",
            _ => "a contextual relationship",
        },
        None => return intensity_only_aspect_hint_phrase(aspect).to_string(),
    };

    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => format!("{base} with extra emphasis"),
        Some("obsessive_focus") => format!("{base} with intensified focus"),
        Some(_) => format!("{base} with extra intensity"),
        None => base.to_string(),
    }
}

/// Fonction intensity_only_aspect_hint_phrase.
fn intensity_only_aspect_hint_phrase(aspect: &AspectFact) -> &'static str {
    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => "an amplifying contact",
        Some("obsessive_focus") => "an intensified focus",
        Some(_) => "an intensified contact",
        None => dynamic_quality_aspect_hint_phrase(aspect),
    }
}

/// Fonction dynamic_quality_aspect_hint_phrase.
fn dynamic_quality_aspect_hint_phrase(aspect: &AspectFact) -> &'static str {
    match aspect_dynamic_quality(aspect) {
        "flow" => "a flow",
        "tension" => "a tension",
        "adjustment" => "an adjustment",
        "intensification" => "an intensified contact",
        _ => "a relationship",
    }
}
