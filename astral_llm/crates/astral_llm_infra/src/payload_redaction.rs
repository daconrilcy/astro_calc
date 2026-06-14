use serde_json::Value;

const SENSITIVE_KEYS: &[&str] = &[
    "birth_date",
    "birth_time",
    "birth_place",
    "birth_datetime",
    "latitude",
    "longitude",
    "lat",
    "lon",
    "lng",
    "coordinates",
    "place_name",
    "city",
    "custom_instructions",
];

pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, val) in map {
                if is_sensitive_key(key) {
                    out.insert(key.clone(), Value::String("[REDACTED]".into()));
                } else {
                    out.insert(key.clone(), redact_value(val));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        other => other.clone(),
    }
}

pub fn redact_request_for_storage(request: &astral_llm_domain::GenerateReadingRequest) -> Value {
    let value = serde_json::to_value(request).unwrap_or_else(|_| serde_json::json!({}));
    redact_value(&value)
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    SENSITIVE_KEYS
        .iter()
        .any(|k| lower == *k || lower.contains(k))
}
