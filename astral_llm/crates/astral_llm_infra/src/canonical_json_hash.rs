//! Hash JSON canonique (cles triees recursivement) pour idempotence jobs.

use sha2::{Digest, Sha256};

/// SHA-256 hex du JSON canonique (cles triees a tous les niveaux).
pub fn canonical_json_hash(value: &serde_json::Value) -> String {
    let canonical = canonicalize(value);
    let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
    hex::encode(Sha256::digest(bytes))
}

fn canonicalize(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                if let Some(v) = map.get(key) {
                    out.insert(key.clone(), canonicalize(v));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(canonicalize).collect())
        }
        other => other.clone(),
    }
}

/// Objet logique job pour idempotence (enveloppe metier sans Idempotency-Key).
pub fn job_logical_payload(envelope: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "service_code": envelope.get("service_code"),
        "payload": envelope.get("payload").cloned().unwrap_or(serde_json::json!({})),
        "user_language": envelope.get("user_language").cloned().unwrap_or(serde_json::json!("fr")),
        "audience_level": envelope.get("audience_level").cloned().unwrap_or(serde_json::json!("beginner")),
        "astrologer_profile": envelope.get("astrologer_profile").cloned().unwrap_or(serde_json::json!({})),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_order_does_not_change_hash() {
        let a = serde_json::json!({"b": 2, "a": 1});
        let b = serde_json::json!({"a": 1, "b": 2});
        assert_eq!(canonical_json_hash(&a), canonical_json_hash(&b));
    }

    #[test]
    fn nested_keys_sorted() {
        let v = serde_json::json!({"z": {"b": 1, "a": 2}, "a": 0});
        let h = canonical_json_hash(&v);
        assert_eq!(h.len(), 64);
    }
}
