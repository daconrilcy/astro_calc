//! Parse object_code / house depuis fact_id et raw_value.

use std::collections::HashMap;

use astral_llm_domain::{astro_fact::NormalizedAstroFacts, generation_response::ReadingChapter};
use astral_llm_infra::CanonicalCatalog;

use crate::astro_label_humanizer::AstroLabelHumanizer;

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
        Some("ruler") if parts.len() >= 4 && parts[1] == "angle" => {
            vec![parts[2].to_string(), parts[parts.len() - 1].to_string()]
        }
        Some("ruler") if parts.len() >= 3 && parts[1] == "ascendant" => {
            vec!["ascendant".into(), parts[parts.len() - 1].to_string()]
        }
        Some("ruler") if parts.len() >= 4 => vec![parts[parts.len() - 1].to_string()],
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
        Some("dominant_planet") if parts.len() >= 2 => vec![parts[1].to_string()],
        Some("dominant_planet") => vec![],
        Some("signal") if parts.len() >= 2 => {
            let key = parts[1];
            if key.starts_with("aspect:") {
                vec![]
            } else {
                key.split(':')
                    .next()
                    .map(str::to_string)
                    .into_iter()
                    .collect()
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
    fact_id.to_lowercase().contains(&format!(":{object}:"))
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
pub fn placement_index_by_object(
    facts: &[astral_llm_domain::NormalizedAstroFact],
) -> HashMap<String, String> {
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
    if let Some(key) = aspect_semantic_key_from_fact_id(fact_id) {
        return key;
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

const SIGNAL_KIND_SEGMENTS: &[&str] = &["aspect", "object_position", "angle", "dignity", "cluster"];

/// Alias connus quand le modele omet le segment `aspect` (ex. `signal:jupiter:uranus:opposition`).
pub fn candidate_fact_id_aliases(fact_id: &str) -> Vec<String> {
    let mut out = Vec::new();
    if let Some((object, suffix)) = fact_id.split_once('.') {
        if suffix == "sign" && !object.is_empty() {
            out.push(format!("placement:{object}"));
        }
    }
    let parts: Vec<&str> = fact_id.split(':').collect();
    if parts.len() == 4 && parts[0] == "signal" && !SIGNAL_KIND_SEGMENTS.contains(&parts[1]) {
        out.push(format!(
            "signal:aspect:{}:{}:{}",
            parts[1], parts[2], parts[3]
        ));
    }
    if parts.len() == 4 && parts[0] == "aspect" {
        out.push(format!(
            "signal:aspect:{}:{}:{}",
            parts[1], parts[2], parts[3]
        ));
    }
    out
}

/// Resout un fact_id cite par le LLM vers l'id canonique du catalogue de faits.
pub fn resolve_canonical_fact_id(fact_id: &str, facts: &NormalizedAstroFacts) -> Option<String> {
    if facts.contains_fact(fact_id) {
        return Some(fact_id.to_string());
    }
    candidate_fact_id_aliases(fact_id)
        .into_iter()
        .find(|candidate| facts.contains_fact(candidate))
}

/// Reecrit les fact_id derives connus avant validation astro_basis.
pub fn normalize_chapter_astro_basis_fact_ids(
    chapter: &mut ReadingChapter,
    facts: &NormalizedAstroFacts,
) {
    for basis in &mut chapter.astro_basis {
        let Some(id) = basis.fact_id.as_ref() else {
            continue;
        };
        let Some(resolved) = resolve_canonical_fact_id(id, facts) else {
            continue;
        };
        if resolved != *id {
            tracing::warn!(
                chapter = %chapter.code,
                received = %id,
                normalized = %resolved,
                "astro_basis fact_id normalized after provider drift"
            );
            basis.fact_id = Some(resolved);
        }
    }
}

pub fn normalize_chapter_astro_basis_fact_ids_with_catalog(
    chapter: &mut ReadingChapter,
    facts: &NormalizedAstroFacts,
    catalog: &CanonicalCatalog,
    language: &str,
) {
    let locale = AstroLabelHumanizer::locale_key(language);
    for basis in &mut chapter.astro_basis {
        let Some(id) = basis.fact_id.as_ref() else {
            continue;
        };
        let Some(resolved) = resolve_canonical_fact_id(id, facts)
            .or_else(|| canonicalize_public_fact_id(id, catalog, locale))
        else {
            continue;
        };
        if resolved != *id {
            tracing::warn!(
                chapter = %chapter.code,
                received = %id,
                normalized = %resolved,
                "astro_basis fact_id normalized after provider drift"
            );
            basis.fact_id = Some(resolved);
        }
    }
}

fn canonicalize_public_fact_id(
    fact_id: &str,
    catalog: &CanonicalCatalog,
    locale: &str,
) -> Option<String> {
    if let Some(rest) = fact_id.strip_prefix("placement:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() >= 4 && parts[parts.len() - 2] == "house" {
            let house = parts[parts.len() - 1];
            let sign = parts.get(parts.len() - 3)?.trim();
            let object = parts[..parts.len() - 3].join(":");
            let object_code = canonical_object_code(&object, catalog, locale);
            let sign_code = canonical_sign_code(sign, catalog, locale);
            return Some(format!("placement:{object_code}:{sign_code}:house:{house}"));
        }
    }
    if let Some(rest) = fact_id.strip_prefix("angle:") {
        let mut parts = rest.split(':');
        let angle = parts.next()?.trim();
        let sign = parts.next()?.trim();
        let angle_code = canonical_object_code(angle, catalog, locale);
        let sign_code = canonical_sign_code(sign, catalog, locale);
        return Some(format!("angle:{angle_code}:{sign_code}"));
    }
    if let Some(rest) = fact_id.strip_prefix("signal:angle:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() >= 3 && parts[1] == "sign" {
            let angle_code = canonical_object_code(parts[0].trim(), catalog, locale);
            let sign_code = canonical_sign_code(parts[2].trim(), catalog, locale);
            return Some(format!("signal:angle:{angle_code}:sign:{sign_code}"));
        }
    }
    if let Some(rest) = fact_id.strip_prefix("ruler:angle:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() >= 2 {
            let angle_code = canonical_object_code(parts[0].trim(), catalog, locale);
            let ruler_code = canonical_object_code(parts[1].trim(), catalog, locale);
            return Some(format!("ruler:angle:{angle_code}:{ruler_code}"));
        }
    }
    if let Some(rest) = fact_id.strip_prefix("ruler:ascendant:") {
        let ruler_code = canonical_object_code(rest.trim(), catalog, locale);
        return Some(format!("ruler:ascendant:{ruler_code}"));
    }
    if let Some(rest) = fact_id.strip_prefix("ruler:dominant_house:") {
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() >= 2 {
            let ruler_code = canonical_object_code(parts[parts.len() - 1].trim(), catalog, locale);
            let prefix = parts[..parts.len() - 1].join(":");
            return Some(format!("ruler:dominant_house:{prefix}:{ruler_code}"));
        }
    }
    if let Some(rest) = fact_id.strip_prefix("dominant_planet:") {
        let ruler_code = canonical_object_code(rest.trim(), catalog, locale);
        return Some(format!("dominant_planet:{ruler_code}"));
    }
    None
}

fn canonical_object_code(raw: &str, catalog: &CanonicalCatalog, locale: &str) -> String {
    catalog
        .object_code_for_label(locale, raw)
        .or_else(|| catalog.object_code_for_label("en", raw))
        .unwrap_or_else(|| raw.trim().to_lowercase())
}

fn canonical_sign_code(raw: &str, catalog: &CanonicalCatalog, locale: &str) -> String {
    catalog
        .sign_code_for_label(locale, raw)
        .or_else(|| catalog.sign_code_for_label("en", raw))
        .unwrap_or_else(|| raw.trim().to_lowercase())
}

fn aspect_semantic_key_from_fact_id(fact_id: &str) -> Option<String> {
    let parts: Vec<&str> = fact_id.split(':').collect();
    let (obj_a, obj_b, aspect_type) = if parts.len() >= 5
        && parts[0] == "signal"
        && parts[1] == "aspect"
    {
        (parts[2], parts[3], parts[4])
    } else if parts.len() == 4 && parts[0] == "signal" && !SIGNAL_KIND_SEGMENTS.contains(&parts[1])
    {
        (parts[1], parts[2], parts[3])
    } else if parts.len() == 4 && parts[0] == "aspect" {
        (parts[1], parts[2], parts[3])
    } else {
        return None;
    };
    let mut pair = [obj_a.to_string(), obj_b.to_string()];
    pair.sort();
    Some(format!("aspect:{}:{}:{}", pair[0], pair[1], aspect_type))
}

fn placement_key_from_evidence(object: &str, raw: &serde_json::Value) -> Option<String> {
    let evidence = raw.get("evidence")?;
    let sign = evidence.get("sign_code").and_then(|v| v.as_str())?;
    let house = evidence.get("house_number").and_then(|v| v.as_u64())?;
    Some(format!("placement:{object}:{sign}:house:{house}"))
}

pub fn matches_requirement_object(fact_id: &str, object_code: Option<&str>, code: &str) -> bool {
    if fact_id.contains(&format!(":{code}:")) || fact_id.ends_with(&format!(":{code}")) {
        return true;
    }
    object_code == Some(code)
}
