use serde_json::json;

use crate::dignities::EssentialDignityFact;

use super::constants::{THEME_FUNCTIONAL_CHALLENGE, THEME_FUNCTIONAL_STRENGTH};
use super::tags::{dedupe_tags, sign_tags};
use super::utils::{indefinite_article, round4};

pub(super) fn dignity_priority(dignity: &EssentialDignityFact) -> f64 {
    let base: f64 = match dignity.object_code.as_str() {
        "sun" | "moon" => 90.0,
        "mercury" | "venus" | "mars" => 86.0,
        "jupiter" | "saturn" => 82.0,
        _ => 72.0,
    };
    let type_delta: f64 = match dignity.dignity_type.as_str() {
        "domicile" => 6.0,
        "exaltation" => 4.0,
        "detriment" => 2.0,
        "fall" => 1.0,
        _ => 0.0,
    };
    round4((base + type_delta).min(95.0))
}

pub(super) fn dignity_title(dignity: &EssentialDignityFact) -> String {
    if dignity.polarity == "dignity" {
        format!(
            "{} strongly placed in {}",
            dignity.object_name, dignity.sign_name
        )
    } else {
        format!(
            "{} under pressure in {}",
            dignity.object_name, dignity.sign_name
        )
    }
}

pub(super) fn dignity_summary(dignity: &EssentialDignityFact) -> String {
    if dignity.polarity == "dignity" {
        format!(
            "{} is in {}, a sign where its function is reinforced by {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        )
    } else {
        format!(
            "{} is in {}, a sign where its function needs more adjustment because of {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        )
    }
}

pub(super) fn dignity_interpretive_hint(dignity: &EssentialDignityFact) -> String {
    let article = indefinite_article(&dignity.dignity_type);
    format!(
        "Treat {} in {} as {} {} modifier for the existing placement signal.",
        dignity.object_name, dignity.sign_name, article, dignity.dignity_type
    )
}

pub(super) fn dignity_effect_phrase(dignity: &EssentialDignityFact) -> &'static str {
    match dignity.dignity_type.as_str() {
        "domicile" => "functional strength, coherence, and self-command",
        "exaltation" => "heightened visibility and constructive emphasis",
        "detriment" => "a need for translation, adaptation, and deliberate handling",
        "fall" => "a more sensitive or constrained expression that needs care",
        _ => "additional interpretive context",
    }
}

pub(super) fn dignity_semantic_tags(dignity: &EssentialDignityFact) -> Vec<String> {
    let mut tags = vec![
        "dignity".to_string(),
        dignity.object_code.clone(),
        dignity.sign_code.clone(),
        dignity.dignity_type.clone(),
    ];
    if dignity.polarity == "dignity" {
        tags.push(THEME_FUNCTIONAL_STRENGTH.to_string());
    } else {
        tags.push(THEME_FUNCTIONAL_CHALLENGE.to_string());
    }
    tags.extend(sign_tags(&dignity.sign_code));
    dedupe_tags(tags)
}

pub(super) fn dignity_evidence(dignity: &EssentialDignityFact) -> serde_json::Value {
    json!({
        "fact_type": "essential_dignity",
        "chart_object_id": dignity.chart_object_id,
        "chart_object": dignity.object_code,
        "object_name": dignity.object_name,
        "sign_id": dignity.sign_id,
        "sign_code": dignity.sign_code,
        "sign_name": dignity.sign_name,
        "dignity_type": dignity.dignity_type,
        "dignity_label": dignity.dignity_label,
        "polarity": dignity.polarity,
        "strength_score": dignity.strength_score,
        "is_major": dignity.is_major
    })
}

pub(super) fn dignity_evidence_array(dignities: &[EssentialDignityFact]) -> serde_json::Value {
    serde_json::Value::Array(dignities.iter().map(dignity_evidence).collect())
}
