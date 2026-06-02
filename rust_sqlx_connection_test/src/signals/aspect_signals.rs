use serde_json::json;

use crate::domain::AspectFact;

use super::constants::BASIC_ASPECT_MIN_STRENGTH;
use super::tags::dedupe_tags;

pub(super) fn aspect_semantic_tags(aspect: &AspectFact, strength_score: f64) -> Vec<String> {
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
    } else if strength_score < BASIC_ASPECT_MIN_STRENGTH {
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
        "is_intensity_modifier": aspect.valence_is_intensity_modifier,
        "writing_guidance": aspect.valence_writing_guidance
            .as_deref()
            .unwrap_or_else(|| aspect_default_writing_guidance(aspect))
    })
}

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

pub(super) fn aspect_writing_guidance(aspect: &AspectFact) -> String {
    let base = aspect
        .valence_writing_guidance
        .as_deref()
        .unwrap_or_else(|| aspect_default_writing_guidance(aspect));

    match aspect.intensity_modifier.as_deref() {
        Some(modifier) if aspect.primary_valence.is_none() => format!(
            "{base} Treat {modifier} as an intensity modifier, not as a supportive or challenging valence by itself."
        ),
        Some(modifier) => format!(
            "{base} Use {modifier} only as an intensity modifier layered onto the primary valence."
        ),
        None => base.to_string(),
    }
}

pub(super) fn aspect_interpretive_hint(aspect: &AspectFact, aspect_name: &str) -> String {
    format!(
        "Read this {aspect_name} as {} between {} and {}, with attention to the {} phase.",
        aspect_hint_quality_phrase(aspect),
        aspect.source_object_name,
        aspect.target_object_name,
        aspect.phase_state
    )
}

fn aspect_hint_quality_phrase(aspect: &AspectFact) -> String {
    let base = match aspect.primary_valence.as_deref() {
        Some("supportive") => "a supportive flow",
        Some("harmonious") => "a natural flow",
        Some("creative" | "refined_creative" | "creative_ordering") => "a creative opening",
        Some("dynamic_challenging") => "an active tension",
        Some("polarizing") => "a polarity to balance",
        Some("minor_friction") => "manageable friction",
        Some("indirect_tension") => "indirect tension",
        Some("adjustment") => "an adjustment",
        Some("subtle_adjustment") => "a subtle adjustment",
        Some("symbolic_fated") => "a symbolic emphasis",
        Some("spiritual_integration") => "an integrating connection",
        Some(_) => "a contextual relationship",
        None => return intensity_only_aspect_hint_phrase(aspect).to_string(),
    };

    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => format!("{base} with extra emphasis"),
        Some("obsessive_focus") => format!("{base} with intensified focus"),
        Some(_) => format!("{base} with extra intensity"),
        None => base.to_string(),
    }
}

fn intensity_only_aspect_hint_phrase(aspect: &AspectFact) -> &'static str {
    match aspect.intensity_modifier.as_deref() {
        Some("amplifying") => "an amplifying contact",
        Some("obsessive_focus") => "an intensified focus",
        Some(_) => "an intensified contact",
        None => dynamic_quality_aspect_hint_phrase(aspect),
    }
}

fn dynamic_quality_aspect_hint_phrase(aspect: &AspectFact) -> &'static str {
    match aspect_dynamic_quality(aspect) {
        "flow" => "a flow",
        "tension" => "a tension",
        "adjustment" => "an adjustment",
        "intensification" => "an intensified contact",
        _ => "a relationship",
    }
}

fn aspect_default_writing_guidance(aspect: &AspectFact) -> &'static str {
    match aspect_dynamic_quality(aspect) {
        "flow" => {
            "Describe ease or cooperation between the two chart factors without presenting it as an automatic benefit."
        }
        "tension" => {
            "Describe the tension between the two chart factors without making it unstable or negative by default."
        }
        "adjustment" => {
            "Describe this as an adjustment between the two chart factors, with practical recalibration rather than blame."
        }
        "intensification" => {
            "Describe this as intensified contact between the two chart factors, and use the planets involved to qualify the tone."
        }
        _ => "Use the aspect as a relationship between two chart factors, not as a standalone verdict.",
    }
}
