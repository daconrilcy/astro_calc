//! Module astral_calculator\src\features\natal\signals\dignity_helpers.rs du moteur astral_calculator.

use serde_json::json;

use crate::domain::ObjectPositionFact;
use crate::features::natal::catalog::BasicPayloadCatalog;
use crate::features::natal::dignities::{dignity_priority_delta, EssentialDignityFact};

use super::constants::{THEME_FUNCTIONAL_CHALLENGE, THEME_FUNCTIONAL_STRENGTH};
use super::positions::object_signal_scoring_number;
use super::tags::{dedupe_tags, sign_tags};
use super::utils::{indefinite_article, round4};

pub(super) fn dignity_priority(
    dignity: &EssentialDignityFact,
    position: &ObjectPositionFact,
    catalog: &BasicPayloadCatalog,
) -> f64 {
    let base = object_signal_scoring_number(position, "position_priority_base").unwrap_or(0.0);
    round4((base + dignity_priority_delta(dignity, catalog)).min(95.0))
}

pub(super) fn dignity_title(dignity: &EssentialDignityFact, locale: &str) -> String {
    match (locale, dignity.polarity.as_str()) {
        ("fr", "dignity") => format!(
            "{} bien placé en {}",
            dignity.object_name, dignity.sign_name
        ),
        ("fr", _) => format!(
            "{} sous tension en {}",
            dignity.object_name, dignity.sign_name
        ),
        ("es", "dignity") => format!(
            "{} bien situado en {}",
            dignity.object_name, dignity.sign_name
        ),
        ("es", _) => format!(
            "{} bajo tensión en {}",
            dignity.object_name, dignity.sign_name
        ),
        ("de", "dignity") => format!(
            "{} stark gestellt in {}",
            dignity.object_name, dignity.sign_name
        ),
        ("de", _) => format!(
            "{} unter Druck in {}",
            dignity.object_name, dignity.sign_name
        ),
        _ if dignity.polarity == "dignity" => format!(
            "{} strongly placed in {}",
            dignity.object_name, dignity.sign_name
        ),
        _ => format!(
            "{} under pressure in {}",
            dignity.object_name, dignity.sign_name
        ),
    }
}

pub(super) fn dignity_summary(dignity: &EssentialDignityFact, locale: &str) -> String {
    match (locale, dignity.polarity.as_str()) {
        ("fr", "dignity") => format!(
            "{} se trouve en {}, un signe où sa fonction est renforcée par {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        ("fr", _) => format!(
            "{} se trouve en {}, un signe où sa fonction demande davantage d'ajustement à cause de {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        ("es", "dignity") => format!(
            "{} está en {}, un signo donde su función se refuerza por {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        ("es", _) => format!(
            "{} está en {}, un signo donde su función necesita más ajuste por {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        ("de", "dignity") => format!(
            "{} befindet sich in {}, einem Zeichen, in dem seine Funktion durch {} gestärkt wird.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        ("de", _) => format!(
            "{} befindet sich in {}, einem Zeichen, in dem seine Funktion wegen {} mehr Anpassung braucht.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        _ if dignity.polarity == "dignity" => format!(
            "{} is in {}, a sign where its function is reinforced by {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
        _ => format!(
            "{} is in {}, a sign where its function needs more adjustment because of {}.",
            dignity.object_name, dignity.sign_name, dignity.dignity_type
        ),
    }
}

pub(super) fn dignity_interpretive_hint(dignity: &EssentialDignityFact, locale: &str) -> String {
    let article = indefinite_article(&dignity.dignity_type);
    match locale {
        "fr" => format!(
            "Traitez {} en {} comme un modificateur {} {} du signal de position existant.",
            dignity.object_name, dignity.sign_name, article, dignity.dignity_type
        ),
        "es" => format!(
            "Trate {} en {} como un modificador {} {} de la señal de posición existente.",
            dignity.object_name, dignity.sign_name, article, dignity.dignity_type
        ),
        "de" => format!(
            "Betrachten Sie {} in {} als einen {} {}-Modifikator für das bestehende Positionssignal.",
            dignity.object_name, dignity.sign_name, article, dignity.dignity_type
        ),
        _ => format!(
            "Treat {} in {} as {} {} modifier for the existing placement signal.",
            dignity.object_name, dignity.sign_name, article, dignity.dignity_type
        ),
    }
}

pub(super) fn dignity_effect_phrase(dignity: &EssentialDignityFact, locale: &str) -> &'static str {
    match (locale, dignity.dignity_type.as_str()) {
        ("fr", "domicile") => "force fonctionnelle, cohérence et maîtrise de soi",
        ("fr", "exaltation") => "visibilité accrue et accent constructif",
        ("fr", "detriment") => "besoin de traduction, d'adaptation et de gestion volontaire",
        ("fr", "fall") => "expression plus sensible ou contrainte qui demande de l'attention",
        ("es", "domicile") => "fuerza funcional, coherencia y autocontrol",
        ("es", "exaltation") => "mayor visibilidad y énfasis constructivo",
        ("es", "detriment") => "necesidad de traducción, adaptación y manejo deliberado",
        ("es", "fall") => "una expresión más sensible o limitada que requiere cuidado",
        ("de", "domicile") => "funktionale Stärke, Kohärenz und Selbststeuerung",
        ("de", "exaltation") => "erhöhte Sichtbarkeit und konstruktive Betonung",
        ("de", "detriment") => "Bedarf an Übersetzung, Anpassung und bewusster Handhabung",
        ("de", "fall") => "ein empfindlicherer oder eingeschränkter Ausdruck, der Sorgfalt braucht",
        (_, "domicile") => "functional strength, coherence, and self-command",
        (_, "exaltation") => "heightened visibility and constructive emphasis",
        (_, "detriment") => "a need for translation, adaptation, and deliberate handling",
        (_, "fall") => "a more sensitive or constrained expression that needs care",
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
