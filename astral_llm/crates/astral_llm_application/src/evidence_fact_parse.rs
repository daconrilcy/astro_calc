//! Parse object_code / house depuis fact_id et raw_value.

use std::collections::HashMap;

pub fn object_code_from_fact_id(fact_id: &str) -> Option<String> {
    object_codes_from_fact_id(fact_id).into_iter().next()
}

/// Famille de fact_id pour l'appariement des roles pack (evite signal:sun -> placement:sun core).
pub fn fact_id_role_bucket(fact_id: &str) -> &'static str {
    if fact_id.starts_with("signal:object_position:") {
        return "signal_object_position";
    }
    if fact_id.starts_with("signal:aspect:") {
        return "signal_aspect";
    }
    if fact_id.starts_with("signal:angle:") {
        return "signal_angle";
    }
    if fact_id.starts_with("signal:dignity:") {
        return "signal_dignity";
    }
    if fact_id.starts_with("placement:") {
        return "placement";
    }
    if fact_id.starts_with("angle:") {
        return "angle";
    }
    if fact_id.starts_with("ruler:") {
        return "ruler";
    }
    "other"
}

/// Corps planetaires / angles cites par un fact_id (placement, aspect, signal, angle).
pub fn object_codes_from_fact_id(fact_id: &str) -> Vec<String> {
    let parts: Vec<&str> = fact_id.split(':').collect();
    match parts.first().copied() {
        Some("placement") => placement_object_code(&parts).into_iter().collect(),
        Some("angle") if parts.len() >= 2 => vec![parts[1].to_string()],
        Some("ruler") if parts.len() >= 4 => vec![parts[parts.len() - 1].to_string()],
        Some("ruler") if parts.len() >= 3 && parts[1] == "ascendant" => vec!["ascendant".into()],
        Some("ruler") if parts.len() >= 2 => vec![parts[1].to_string()],
        Some("signal") if parts.len() >= 5 && parts[1] == "aspect" => {
            vec![parts[2].to_string(), parts[3].to_string()]
        }
        Some("signal") if parts.len() >= 5 && parts[1] == "angle" && parts[3] == "sign" => {
            vec![parts[2].to_string()]
        }
        Some("signal") if parts.len() >= 3 && parts[1] == "object_position" => {
            vec![parts[2].to_string()]
        }
        Some("signal") if parts.len() >= 4 && parts[1] == "dignity" => vec![parts[2].to_string()],
        Some("signal") if parts.len() >= 2 => {
            let key = parts[1];
            if key.starts_with("aspect:") {
                vec![]
            } else {
                key.split(':').next().map(str::to_string).into_iter().collect()
            }
        }
        _ => vec![],
    }
}

fn placement_object_code(parts: &[&str]) -> Option<String> {
    if parts.len() < 2 {
        return None;
    }
    if parts.len() >= 5 && parts[parts.len() - 2] == "house" {
        let sign_idx = parts.len() - 3;
        if sign_idx == 2 {
            return Some(parts[1].to_string());
        }
        return Some(parts[1..sign_idx].join(":"));
    }
    if let Some(i) = parts.iter().position(|&p| p == "house").filter(|&i| i > 1) {
        return Some(parts[1..i].join(":"));
    }
    Some(parts[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_object_codes_standard() {
        assert_eq!(
            object_codes_from_fact_id("placement:jupiter:cancer:house:8"),
            vec!["jupiter"]
        );
        assert_eq!(
            object_codes_from_fact_id("signal:aspect:jupiter:uranus:opposition"),
            vec!["jupiter", "uranus"]
        );
        assert_eq!(
            object_codes_from_fact_id("signal:angle:ascendant:sign:scorpio"),
            vec!["ascendant"]
        );
        assert_eq!(
            fact_id_role_bucket("signal:object_position:sun"),
            "signal_object_position"
        );
        assert_eq!(
            fact_id_role_bucket("placement:sun:capricorn:house:2"),
            "placement"
        );
    }

    #[test]
    fn semantic_key_aligns_signal_and_placement_sun() {
        let mut placements = HashMap::new();
        placements.insert(
            "sun".into(),
            "placement:sun:capricorn:house:2".into(),
        );
        let signal = compute_semantic_fact_key(
            "signal:object_position:sun",
            &serde_json::json!({}),
            &placements,
        );
        assert_eq!(signal, "placement:sun:capricorn:house:2");
    }
}

pub fn house_number_from_fact(fact_id: &str, raw: &serde_json::Value) -> Option<u8> {
    if let Some(h) = raw.get("source_house_number").and_then(|v| v.as_u64()) {
        return u8::try_from(h).ok();
    }
    if let Some(h) = raw.get("house").and_then(|v| v.as_u64()) {
        return u8::try_from(h).ok();
    }
    let parts: Vec<&str> = fact_id.split(':').collect();
    if parts.first() == Some(&"placement") {
        for (i, p) in parts.iter().enumerate() {
            if *p == "house" && i + 1 < parts.len() {
                return parts[i + 1].parse().ok();
            }
        }
    }
    None
}

pub fn fact_involves_object(fact_id: &str, object: &str) -> bool {
    fact_id
        .to_lowercase()
        .contains(&format!(":{object}:"))
        || fact_id.ends_with(&format!(":{object}"))
        || object_code_from_fact_id(fact_id).is_some_and(|o| o == object)
}

pub fn fact_involves_house(fact_id: &str, raw: &serde_json::Value, house: u8) -> bool {
    house_number_from_fact(fact_id, raw) == Some(house)
        || fact_id.contains(&format!("house:{house}"))
        || fact_id.contains(&format!("maison {house}"))
}

pub fn aspect_involves_object(fact_id: &str, label: &str, object: &str) -> bool {
    let blob = format!("{fact_id} {label}").to_lowercase();
    blob.contains(object)
}

pub fn sign_code_from_fact(fact_id: &str, raw: &serde_json::Value) -> Option<String> {
    if let Some(s) = raw.get("sign").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    if let Some(s) = raw.pointer("/evidence/sign_code").and_then(|v| v.as_str()) {
        return Some(s.to_string());
    }
    let parts: Vec<&str> = fact_id.split(':').collect();
    if parts.first() == Some(&"placement") && parts.len() >= 3 {
        return Some(parts[2].to_string());
    }
    if parts.len() >= 5 && parts[1] == "angle" && parts[3] == "sign" {
        return Some(parts[4].to_string());
    }
    None
}

/// Index objet -> fact_id placement canonique (premier gagnant).
pub fn placement_index_by_object(facts: &[astral_llm_domain::NormalizedAstroFact]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for fact in facts {
        if !fact.id.starts_with("placement:") {
            continue;
        }
        if let Some(obj) = object_code_from_fact_id(&fact.id) {
            map.entry(obj).or_insert_with(|| fact.id.clone());
        }
    }
    map
}

/// Cle interpretative stable : aligne signal:object_position:* sur placement:* equivalent.
pub fn compute_semantic_fact_key(
    fact_id: &str,
    raw: &serde_json::Value,
    placement_by_object: &HashMap<String, String>,
) -> String {
    if fact_id.starts_with("placement:") || fact_id.starts_with("ruler:") {
        return fact_id.to_string();
    }
    if let Some(obj) = fact_id.strip_prefix("signal:object_position:") {
        if let Some(placement_id) = placement_by_object.get(obj) {
            return placement_id.clone();
        }
        if let Some(key) = placement_key_from_evidence(obj, raw) {
            return key;
        }
        return format!("object_position:{obj}");
    }
    if fact_id.starts_with("signal:aspect:") {
        let parts: Vec<&str> = fact_id.split(':').collect();
        if parts.len() >= 5 {
            let mut pair = [parts[2].to_string(), parts[3].to_string()];
            pair.sort();
            return format!("aspect:{}:{}:{}", pair[0], pair[1], parts[4]);
        }
    }
    if fact_id.starts_with("signal:angle:") {
        let parts: Vec<&str> = fact_id.split(':').collect();
        if parts.len() >= 5 && parts[3] == "sign" {
            return format!("angle:{}:{}", parts[2], parts[4]);
        }
    }
    if fact_id.starts_with("angle:") {
        return fact_id.to_string();
    }
    fact_id.to_string()
}

fn placement_key_from_evidence(object: &str, raw: &serde_json::Value) -> Option<String> {
    let evidence = raw.get("evidence")?;
    let sign = evidence.get("sign_code").and_then(|v| v.as_str())?;
    let house = evidence.get("house_number").and_then(|v| v.as_u64())?;
    Some(format!("placement:{object}:{sign}:house:{house}"))
}

pub fn matches_requirement_object(
    fact_id: &str,
    object_code: Option<&str>,
    code: &str,
) -> bool {
    if fact_id.contains(&format!(":{code}:")) || fact_id.ends_with(&format!(":{code}")) {
        return true;
    }
    object_code == Some(code)
}
