//! Extraction de faits astro normalises depuis les contrats entrant.

use astral_llm_domain::astro_fact::{AstroFactKind, AstroFactUsage, NormalizedAstroFact};
use astral_llm_domain::{GenerationError, GenerationErrorCode, PrivacyPolicy};
use astral_llm_infra::payload_redaction::redact_value;

const NATAL_CONTRACT: &str = "natal_structured_v14";
const SIMPLIFIED_CONTRACT: &str = "natal_simplified_structured_v1";
const PROJECTION_CONTRACT: &str = "llm_projection_natal_v1";
const PROJECTION_SIMPLIFIED_CONTRACT: &str = "llm_projection_natal_simplified_v1";

pub fn extract_facts(
    contract_version: &str,
    data: &serde_json::Value,
    privacy: &PrivacyPolicy,
) -> Result<Vec<NormalizedAstroFact>, GenerationError> {
    match contract_version {
        NATAL_CONTRACT => Ok(extract_natal_structured(data, privacy)),
        SIMPLIFIED_CONTRACT => Ok(extract_simplified_structured(data, privacy)),
        PROJECTION_CONTRACT => Ok(extract_llm_projection(data, privacy)),
        PROJECTION_SIMPLIFIED_CONTRACT => Ok(extract_llm_projection_simplified(data, privacy)),
        other => Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("unsupported astro_result.contract_version: {other}"),
            serde_json::json!({
                "known_versions": [
                    NATAL_CONTRACT,
                    SIMPLIFIED_CONTRACT,
                    PROJECTION_CONTRACT,
                    PROJECTION_SIMPLIFIED_CONTRACT
                ]
            }),
        )),
    }
}

fn extract_simplified_structured(
    data: &serde_json::Value,
    privacy: &PrivacyPolicy,
) -> Vec<NormalizedAstroFact> {
    let mut facts = Vec::new();

    if let Some(entries) = data.get("facts").and_then(|v| v.as_array()) {
        for entry in entries {
            let Some(object_code) = entry.get("object_code").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(sign_code) = entry.get("sign_code").and_then(|v| v.as_str()) else {
                continue;
            };
            let reliability = entry
                .get("reliability")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if reliability == "ambiguous_across_uncertainty_window"
                || reliability == "reference_based"
                || reliability == "excluded_missing_input"
            {
                continue;
            }
            let mut value = serde_json::json!({
                "object_code": object_code,
                "sign_code": sign_code,
                "reliability": reliability,
            });
            if privacy.redact_birth_data_before_llm {
                value = redact_value(&value);
            }
            facts.push(NormalizedAstroFact {
                id: format!("placement:{object_code}"),
                kind: AstroFactKind::PlanetPosition,
                kind_code: "planet_sign".to_string(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: format!("{object_code} in {sign_code}"),
                value,
                interpretive_weight: None,
                domains: vec!["identity".to_string()],
            });
        }
    }

    if let Some(planets) = data.get("planets").and_then(|v| v.as_object()) {
        for (planet, detail) in planets {
            if facts
                .iter()
                .any(|fact| fact.id == format!("placement:{planet}"))
            {
                continue;
            }
            extract_legacy_planet(planet, detail, &mut facts, privacy);
        }
    }

    facts
}

fn extract_llm_projection_simplified(
    data: &serde_json::Value,
    _privacy: &PrivacyPolicy,
) -> Vec<NormalizedAstroFact> {
    let _ = data;
    Vec::new()
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
            let sa = a
                .get("priority_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let sb = b
                .get("priority_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });
        for signal in sorted.into_iter().take(48) {
            extract_signal(signal, &mut facts, privacy);
        }
    }

    extract_rulership_context(data, &mut facts, privacy);
    extract_chart_emphasis(data, &mut facts, privacy);
    extract_house_axis_emphasis(data, &mut facts, privacy);
    extract_chart_sect(data, &mut facts, privacy);

    facts
}

fn extract_chart_emphasis(
    data: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let Some(emphasis) = data.get("chart_emphasis").and_then(|v| v.as_object()) else {
        return;
    };

    if let Some(objects) = emphasis.get("dominant_objects").and_then(|v| v.as_array()) {
        for entry in objects.iter().take(3) {
            let object_code = entry
                .get("object_code")
                .and_then(|v| v.as_str())
                .unwrap_or("object");
            let score = entry.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5);
            let mut value = entry.clone();
            if privacy.redact_birth_data_before_llm {
                value = redact_value(&value);
            }
            facts.push(NormalizedAstroFact {
                id: format!("dominant_planet:{object_code}"),
                kind: AstroFactKind::PlanetPosition,
                kind_code: "dominant_planet".to_string(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: format!("Planete dominante : {object_code}"),
                value,
                interpretive_weight: Some(score as f32),
                domains: vec!["synthesis".into()],
            });
        }
    }

    if let Some(houses) = emphasis.get("dominant_houses").and_then(|v| v.as_array()) {
        for entry in houses.iter().take(3) {
            let house_number = entry
                .get("house_number")
                .and_then(|v| v.as_u64())
                .and_then(|n| u8::try_from(n).ok());
            let theme = entry
                .get("theme_code")
                .and_then(|v| v.as_str())
                .unwrap_or("house");
            let score = entry.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5);
            let mut value = entry.clone();
            if privacy.redact_birth_data_before_llm {
                value = redact_value(&value);
            }
            facts.push(NormalizedAstroFact {
                id: format!("house_emphasis:house:{}", house_number.unwrap_or(0)),
                kind: AstroFactKind::HousePlacement,
                kind_code: "house_emphasis".to_string(),
                usage: AstroFactUsage::InterpretiveBasis,
                label: format!("Emphase maison {theme}"),
                value,
                interpretive_weight: Some(score as f32),
                domains: vec!["synthesis".into(), "resources".into()],
            });
        }
    }

    let mut element_scores: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    let mut modality_scores: std::collections::HashMap<String, f64> =
        std::collections::HashMap::new();
    if let Some(signs) = emphasis.get("dominant_signs").and_then(|v| v.as_array()) {
        for entry in signs {
            let sign = entry
                .get("sign_code")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let score = entry.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5);
            if let Some(element) = sign_element_code(sign) {
                *element_scores.entry(element.to_string()).or_insert(0.0) += score;
            }
            if let Some(modality) = sign_modality_code(sign) {
                *modality_scores.entry(modality.to_string()).or_insert(0.0) += score;
            }
        }
    }
    push_balance_fact(
        facts,
        "element_balance",
        &element_scores,
        "Balance elementaire",
        privacy,
    );
    push_balance_fact(
        facts,
        "modality_balance",
        &modality_scores,
        "Balance modale",
        privacy,
    );
}

fn push_balance_fact(
    facts: &mut Vec<NormalizedAstroFact>,
    kind_code: &str,
    scores: &std::collections::HashMap<String, f64>,
    label_prefix: &str,
    privacy: &PrivacyPolicy,
) {
    let Some((winner, score)) = scores
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return;
    };
    let mut value = serde_json::json!({
        "dominant_code": winner,
        "score": score,
        "distribution": scores,
    });
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }
    facts.push(NormalizedAstroFact {
        id: format!("{kind_code}:{winner}"),
        kind: AstroFactKind::HousePlacement,
        kind_code: kind_code.to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("{label_prefix} : {winner}"),
        value,
        interpretive_weight: Some(*score as f32),
        domains: vec!["synthesis".into()],
    });
}

fn extract_house_axis_emphasis(
    data: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let Some(axes) = data.get("house_axis_emphasis").and_then(|v| v.as_array()) else {
        return;
    };
    for axis in axes.iter().take(3) {
        let axis_code = axis
            .get("axis_code")
            .and_then(|v| v.as_str())
            .unwrap_or("axis");
        let mut value = axis.clone();
        if privacy.redact_birth_data_before_llm {
            value = redact_value(&value);
        }
        facts.push(NormalizedAstroFact {
            id: format!("house_axis:{axis_code}"),
            kind: AstroFactKind::HousePlacement,
            kind_code: "house_axis".to_string(),
            usage: AstroFactUsage::InterpretiveBasis,
            label: format!("Axe maison : {axis_code}"),
            value,
            interpretive_weight: Some(0.7),
            domains: vec!["synthesis".into()],
        });
    }
}

fn extract_chart_sect(
    data: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let Some(sect) = data
        .pointer("/chart_context/sect/chart_sect")
        .and_then(|v| v.as_str())
    else {
        return;
    };
    let mut value = data
        .pointer("/chart_context/sect")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "chart_sect": sect }));
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }
    facts.push(NormalizedAstroFact {
        id: format!("sect_condition:{sect}"),
        kind: AstroFactKind::PlanetPosition,
        kind_code: "sect_condition".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("Secte du theme : {sect}"),
        value,
        interpretive_weight: Some(0.6),
        domains: vec!["synthesis".into()],
    });
}

fn sign_element_code(sign: &str) -> Option<&'static str> {
    match sign.to_lowercase().as_str() {
        "aries" | "leo" | "sagittarius" => Some("fire"),
        "taurus" | "virgo" | "capricorn" => Some("earth"),
        "gemini" | "libra" | "aquarius" => Some("air"),
        "cancer" | "scorpio" | "pisces" => Some("water"),
        _ => None,
    }
}

fn sign_modality_code(sign: &str) -> Option<&'static str> {
    match sign.to_lowercase().as_str() {
        "aries" | "cancer" | "libra" | "capricorn" => Some("cardinal"),
        "taurus" | "leo" | "scorpio" | "aquarius" => Some("fixed"),
        "gemini" | "virgo" | "sagittarius" | "pisces" => Some("mutable"),
        _ => None,
    }
}

fn extract_rulership_context(
    data: &serde_json::Value,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let Some(ctx) = data.get("rulership_context") else {
        return;
    };

    let entries: &[(&str, &serde_json::Value)] = &[
        (
            "ascendant_ruler",
            ctx.get("ascendant_ruler")
                .unwrap_or(&serde_json::Value::Null),
        ),
        (
            "mc_ruler",
            ctx.get("mc_ruler").unwrap_or(&serde_json::Value::Null),
        ),
        (
            "descendant_ruler",
            ctx.get("descendant_ruler")
                .unwrap_or(&serde_json::Value::Null),
        ),
    ];

    for (role, entry) in entries {
        if !entry.is_null() {
            push_rulership_fact(entry, role, facts, privacy);
        }
    }

    if let Some(list) = ctx.get("dominant_house_rulers").and_then(|v| v.as_array()) {
        for entry in list.iter().take(6) {
            push_rulership_fact(entry, "dominant_house_ruler", facts, privacy);
        }
    }
}

fn angle_cusp_house_number(source_kind: &str, source_code: &str) -> Option<u8> {
    if source_kind != "angle" {
        return None;
    }
    match source_code {
        "ascendant" => Some(1),
        "descendant" => Some(7),
        "mc" => Some(10),
        "ic" => Some(4),
        _ => None,
    }
}

fn push_rulership_fact(
    entry: &serde_json::Value,
    role: &str,
    facts: &mut Vec<NormalizedAstroFact>,
    privacy: &PrivacyPolicy,
) {
    let ruler_object = entry
        .get("ruler_object_code")
        .and_then(|v| v.as_str())
        .unwrap_or("ruler");
    let source_code = entry
        .get("source_code")
        .and_then(|v| v.as_str())
        .unwrap_or("source");
    let source_kind = entry
        .get("source_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("house");
    let sign_code = entry.get("sign_code").and_then(|v| v.as_str());
    let ruler_house = entry
        .get("ruler_house_number")
        .and_then(|v| v.as_u64())
        .and_then(|n| u8::try_from(n).ok());
    let hint = entry
        .get("interpretive_hint")
        .and_then(|v| v.as_str())
        .unwrap_or("House or angle rulership link");

    let id = format!("ruler:{source_kind}:{source_code}:{ruler_object}");
    let theme = match source_code {
        "ascendant" => vec!["identity".into()],
        "mc" => vec!["career".into()],
        "ic" => vec!["family_roots".into(), "emotional_life".into()],
        "descendant" => vec!["relationships".into()],
        code if code.starts_with("house_") => {
            let house = code.strip_prefix("house_").unwrap_or(code);
            match house {
                "2" => vec!["resources".into()],
                "3" => vec!["communication_mind".into()],
                "4" => vec!["emotional_life".into(), "family_roots".into()],
                "7" => vec!["relationships".into()],
                "8" | "9" | "12" => vec!["growth_path".into()],
                "10" => vec!["career".into()],
                _ => vec![],
            }
        }
        _ => vec![],
    };

    let mut value = serde_json::json!({
        "ruler_object": ruler_object,
        "source_kind": source_kind,
        "source_code": source_code,
        "interpretive_hint": hint,
        "interpretive_role": role,
    });
    if let Some(s) = sign_code {
        value["sign"] = serde_json::json!(s);
    }
    if let Some(h) = ruler_house {
        value["house"] = serde_json::json!(h);
    }
    if let Some(h) = angle_cusp_house_number(source_kind, source_code) {
        value["source_house_number"] = serde_json::json!(h);
    }
    if privacy.redact_birth_data_before_llm {
        value = redact_value(&value);
    }

    facts.push(NormalizedAstroFact {
        id: id.clone(),
        kind: AstroFactKind::Ruler,
        kind_code: "house_ruler".to_string(),
        usage: AstroFactUsage::InterpretiveBasis,
        label: format!("Maitre ({source_code}) : {ruler_object}"),
        value,
        interpretive_weight: Some(0.85),
        domains: theme,
    });
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
        || pos.pointer("/object_context/role").and_then(|v| v.as_str()) == Some("angle")
    {
        AstroFactKind::Angle
    } else {
        AstroFactKind::PlanetPosition
    };

    facts.push(NormalizedAstroFact {
        id: format!(
            "placement:{object_code}:{sign}:house:{}",
            house.unwrap_or(0)
        ),
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
        (
            AstroFactKind::Aspect,
            "aspect",
            AstroFactUsage::InterpretiveBasis,
        )
    } else if signal_key.contains("dignity") {
        (
            AstroFactKind::Dignity,
            "essential_dignity",
            AstroFactUsage::InterpretiveBasis,
        )
    } else if signal_key.starts_with("angle:") {
        (
            AstroFactKind::Angle,
            "angle",
            AstroFactUsage::InterpretiveBasis,
        )
    } else if signal_key.contains("ruler") {
        (
            AstroFactKind::Ruler,
            "house_ruler",
            AstroFactUsage::InterpretiveBasis,
        )
    } else {
        (
            AstroFactKind::PlanetPosition,
            "placement",
            AstroFactUsage::InterpretiveBasis,
        )
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
        domains: if group == "core" { vec![] } else { vec![] },
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
        if !out
            .iter()
            .any(|existing: &NormalizedAstroFact| existing.id == fact.id)
        {
            out.push(fact.with_kind_code());
        }
    }
    out
}
