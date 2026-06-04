//! Extraction de faits astro normalises depuis les contrats entrant.

use astral_llm_domain::astro_fact::{
    AstroFactKind, AstroFactUsage, NormalizedAstroFact,
};
use astral_llm_domain::{GenerationError, GenerationErrorCode, PrivacyPolicy};
use astral_llm_infra::payload_redaction::redact_value;

const NATAL_CONTRACT: &str = "natal_structured_v13";
const PROJECTION_CONTRACT: &str = "llm_projection_natal_v1";

pub fn extract_facts(
    contract_version: &str,
    data: &serde_json::Value,
    privacy: &PrivacyPolicy,
) -> Result<Vec<NormalizedAstroFact>, GenerationError> {
    match contract_version {
        NATAL_CONTRACT => Ok(extract_natal_structured(data, privacy)),
        PROJECTION_CONTRACT => Ok(extract_llm_projection(data, privacy)),
        other => Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("unsupported astro_result.contract_version: {other}"),
            serde_json::json!({ "known_versions": [NATAL_CONTRACT, PROJECTION_CONTRACT] }),
        )),
    }
}

fn extract_natal_structured(
    data: &serde_json::Value,
    privacy: &PrivacyPolicy,
) -> Vec<NormalizedAstroFact> {
    let mut facts = Vec::new();
    extract_domain_scores(data, &mut facts);

    if let Some(positions) = data.get("positions").and_then(|v| v.as_array()) {
        for pos in positions {
            extract_position(pos, &mut facts, privacy);
        }
    }

    if let Some(planets) = data.get("planets").and_then(|v| v.as_object()) {
        for (planet, detail) in planets {
            extract_legacy_planet(planet, detail, &mut facts, privacy);
        }
    }

    if let Some(signals) = data.get("signals").and_then(|v| v.as_array()) {
        let mut sorted: Vec<_> = signals.iter().collect();
        sorted.sort_by(|a, b| {
            let sa = a.get("priority_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let sb = b.get("priority_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        for signal in sorted.into_iter().take(48) {
            extract_signal(signal, &mut facts, privacy);
        }
    }

    facts
}

fn extract_llm_projection(
    data: &serde_json::Value,
    privacy: &PrivacyPolicy,
) -> Vec<NormalizedAstroFact> {
    let mut facts = Vec::new();
    extract_domain_scores(data, &mut facts);

    if let Some(core) = data.get("core_identity").and_then(|v| v.as_object()) {
        for (body, detail) in core {
            extract_projection_core_body(body, detail, &mut facts, privacy);
        }
    }

    if let Some(angles) = data.get("angles").and_then(|v| v.as_object()) {
        for (angle, detail) in angles {
            extract_projection_angle(angle, detail, &mut facts, privacy);
        }
    }

    if let Some(placements) = data.get("placements").and_then(|v| v.as_object()) {
        for (group, items) in placements {
            if let Some(list) = items.as_array() {
                for item in list {
                    extract_projection_placement(group, item, &mut facts, privacy);
                }
            }
        }
    }

    if let Some(aspects) = data
        .pointer("/dynamics/major_aspects")
        .and_then(|v| v.as_array())
    {
        for aspect in aspects.iter().take(8) {
            extract_projection_aspect(aspect, &mut facts, privacy);
        }
    }

    if let Some(axes) = data.get("house_axes").and_then(|v| v.as_array()) {
        for axis in axes.iter().take(4) {
            extract_house_axis(axis, &mut facts, privacy);
        }
    }

    facts
}

fn extract_domain_scores(data: &serde_json::Value, facts: &mut Vec<NormalizedAstroFact>) {
    if let Some(scores) = data.get("domain_scores").and_then(|v| v.as_object()) {
        for (domain, score) in scores {
            if let Some(weight) = score.as_f64() {
                facts.push(NormalizedAstroFact {
                    id: format!("domain_score:{domain}"),
                    kind: AstroFactKind::DomainScore,
                    kind_code: "domain_score".to_string(),
                    usage: AstroFactUsage::DomainSelection,
                    label: format!("Score domaine {domain}"),
                    value: serde_json::json!(weight),
                    interpretive_weight: Some(weight as f32),
                    domains: vec![domain.clone()],
                });
            }
        }
    }
}

fn extract_position(
    pos: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let object_code = pos
        .get("object_code")
        .and_then(|v| v.as_str())
        .unwrap_or("object");
    let object_name = pos
        .get("object_name")
        .and_then(|v| v.as_str())
        .unwrap_or(object_code);
    let sign = pos
        .get("sign_code")
        .or_else(|| pos.get("sign_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let house = pos.get("house_number").and_then(|v| v.as_u64());
    let domain = pos
        .pointer("/house_context/theme_code")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut value = serde_json::json!({
        "object": object_name,
        "sign": sign,
    });
    if let Some(h) = house {
        value["house"] = serde_json::json!(h);
    }
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    let kind = if object_code == "ascendant"
        || object_code.ends_with("coeli")
        || pos.pointer("/object_context/role")
            .and_then(|v| v.as_str())
            == Some("angle")
    {
        AstroFactKind::Angle
    } else {
        AstroFactKind::PlanetPosition
    };

    facts.push(NormalizedAstroFact {
        id: format!("placement:{object_code}:{sign}:house:{}", house.unwrap_or(0)),
        kind,
        kind_code: String::new(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!(
            "{object_name} en {sign}{}",
            house.map(|h| format!(" maison {h}")).unwrap_or_default()
        ),
        value,
        interpretive_weight: None,
        domains: domain.map(|d| vec![d]).unwrap_or_default(),
    });
}

fn extract_legacy_planet(
    planet: &str,
    detail: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let house = detail.get("house").and_then(|v| v.as_u64());
    let sign = detail
        .get("sign")
        .or_else(|| detail.get("sign_code"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let mut value = serde_json::json!({ "object": planet, "sign": sign });
    if let Some(h) = house {
        value["house"] = serde_json::json!(h);
    }
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("placement:{planet}:{sign}:house:{}", house.unwrap_or(0)),
        kind: AstroFactKind::PlanetPosition,
        kind_code: "placement".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!(
            "{planet} en {sign}{}",
            house.map(|h| format!(" maison {h}")).unwrap_or_default()
        ),
        value,
        interpretive_weight: None,
        domains: vec![],
    });
}

fn extract_signal(
    signal: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let signal_key = signal
        .get("signal_key")
        .and_then(|v| v.as_str())
        .unwrap_or("signal");
    let title = signal
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(signal_key);
    let theme = signal
        .get("theme_code")
        .and_then(|v| v.as_str())
        .filter(|t| *t != "aspect")
        .map(|s| s.to_string());

    let (kind, kind_code, usage) = if signal_key.starts_with("aspect:") {
        (AstroFactKind::Aspect, "aspect", AstroFactUsage::InterpretiveBasis)
    } else if signal_key.contains("dignity") {
        (AstroFactKind::Dignity, "essential_dignity", AstroFactUsage::InterpretiveBasis)
    } else if signal_key.starts_with("angle:") {
        (AstroFactKind::Angle, "angle", AstroFactUsage::InterpretiveBasis)
    } else if signal_key.contains("ruler") {
        (AstroFactKind::Ruler, "house_ruler", AstroFactUsage::InterpretiveBasis)
    } else {
        (AstroFactKind::PlanetPosition, "placement", AstroFactUsage::InterpretiveBasis)
    };

    let mut value = serde_json::json!({
        "title": title,
        "summary": signal.get("summary"),
        "interpretive_hint": signal.get("interpretive_hint"),
    });
    if let Some(evidence) = signal.get("evidence") {
        value["evidence"] = evidence.clone();
    }
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("signal:{signal_key}"),
        kind,
        kind_code: kind_code.to_string(),
        usage,
        label: title.to_string(),
        value,
        interpretive_weight: signal
            .get("priority_score")
            .and_then(|v| v.as_f64())
            .map(|s| s as f32),
        domains: theme.map(|d| vec![d]).unwrap_or_default(),
    });
}

fn extract_projection_core_body(
    body: &str,
    detail: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    if body == "ascendant" {
        extract_projection_ascendant(detail, facts, privacy);
        return;
    }

    let Some(placement) = detail.get("placement") else {
        return;
    };
    let sign = placement
        .get("sign")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let house = placement.pointer("/house/number").and_then(|v| v.as_u64());
    let mut value = placement.clone();
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("placement:{body}:{sign}:house:{}", house.unwrap_or(0)),
        kind: AstroFactKind::PlanetPosition,
        kind_code: "placement".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!(
            "{body} en {sign}{}",
            house.map(|h| format!(" maison {h}")).unwrap_or_default()
        ),
        value,
        interpretive_weight: None,
        domains: vec![],
    });
}

fn extract_projection_ascendant(
    detail: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let sign = detail
        .get("sign")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mut value = serde_json::json!({ "sign": sign, "importance": detail.get("importance") });
    if let Some(ruler) = detail.get("ruler") {
        value["ruler"] = ruler.clone();
        if let Some(trad) = ruler.get("traditional").and_then(|v| v.as_str()) {
            facts.push(NormalizedAstroFact {
                id: format!("ruler:ascendant:traditional:{trad}"),
                kind: AstroFactKind::Ruler,
                kind_code: "house_ruler".to_string(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: format!("Maitre traditionnel de l'Ascendant : {trad}"),
                value: serde_json::json!({ "ruler": trad, "sign": sign }),
                interpretive_weight: None,
                domains: vec!["identity".into()],
            });
        }
    }
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("angle:ascendant:{sign}"),
        kind: AstroFactKind::Angle,
        kind_code: "angle".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("Ascendant en {sign}"),
        value,
        interpretive_weight: None,
        domains: vec!["identity".into()],
    });
}

fn extract_projection_angle(
    angle: &str,
    detail: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let sign = detail
        .get("sign")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mut value = detail.clone();
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("angle:{angle}:{sign}"),
        kind: AstroFactKind::Angle,
        kind_code: "angle".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("{angle} en {sign}"),
        value,
        interpretive_weight: None,
        domains: vec![],
    });
}

fn extract_projection_placement(
    group: &str,
    item: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let object = item
        .get("object")
        .and_then(|v| v.as_str())
        .unwrap_or("object");
    let sign = item
        .get("sign")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let house = item.pointer("/house/number").and_then(|v| v.as_u64());
    let mut value = item.clone();
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("placement:{object}:{sign}:house:{}", house.unwrap_or(0)),
        kind: AstroFactKind::PlanetPosition,
        kind_code: "placement".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!(
            "{object} en {sign}{}",
            house.map(|h| format!(" maison {h}")).unwrap_or_default()
        ),
        value,
        interpretive_weight: None,
        domains: if group == "core" {
            vec![]
        } else {
            vec![]
        },
    });
}

fn extract_projection_aspect(
    aspect: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let label = aspect
        .get("aspect")
        .and_then(|v| v.as_str())
        .unwrap_or("aspect");
    let slug = label
        .to_lowercase()
        .replace([' ', ':'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>();
    let mut value = aspect.clone();
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("aspect:{slug}"),
        kind: AstroFactKind::Aspect,
        kind_code: "aspect".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: label.to_string(),
        value,
        interpretive_weight: None,
        domains: vec![],
    });
}

fn extract_house_axis(
    axis: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let theme = axis
        .get("theme")
        .or_else(|| axis.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or("axis");
    let mut value = axis.clone();
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: format!("house_axis:{}", theme.to_lowercase().replace(' ', "_")),
        kind: AstroFactKind::HousePlacement,
        kind_code: "house_axis".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("Axe maison : {theme}"),
        value,
        interpretive_weight: None,
        domains: vec![],
    });
}

pub fn dedupe_facts(facts: Vec<NormalizedAstroFact>) -> Vec<NormalizedAstroFact> {
    let mut out = Vec::new();
    for fact in facts {
        if !out.iter().any(|existing: &NormalizedAstroFact| existing.id == fact.id) {
            out.push(fact.with_kind_code());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_interpretive_placements_from_natal() {
        let data = serde_json::json!({
            "domain_scores": { "identity": 0.8 },
            "planets": {
                "sun": { "house": 2, "sign": "capricorn" },
                "moon": { "house": 4, "sign": "pisces" }
            }
        });
        let facts = extract_natal_structured(&data, &PrivacyPolicy::default());
        assert!(facts.iter().any(|f| f.id.starts_with("placement:sun")));
        assert!(facts.iter().any(|f| f.usage == AstroFactUsage::DomainSelection));
        assert!(facts.iter().any(|f| f.usage == AstroFactUsage::InterpretiveBasis));
    }

    #[test]
    fn signal_fact_retains_evidence_for_post_llm_humanizer() {
        let data = serde_json::json!({
            "signals": [{
                "signal_key": "object_position:moon",
                "title": "Moon in Pisces, house 4",
                "evidence": {
                    "object_code": "moon",
                    "sign_code": "pisces",
                    "house_number": 4
                }
            }]
        });
        let facts = extract_natal_structured(&data, &PrivacyPolicy::default());
        let moon = facts
            .iter()
            .find(|f| f.id == "signal:object_position:moon")
            .expect("moon signal");
        assert_eq!(
            moon.value.pointer("/evidence/sign_code").and_then(|v| v.as_str()),
            Some("pisces")
        );
    }
}
