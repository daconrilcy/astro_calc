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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_request_for_storage_applies_redaction() {
        let request = astral_llm_domain::GenerateReadingRequest {
            request_id: None,
            idempotency_key: None,
            product_context: astral_llm_domain::ProductContext {
                product_code: "natal_prompter".into(),
                interpretation_profile_code: Some("natal_basic".into()),
                user_language: "fr".into(),
                audience_level: astral_llm_domain::AudienceLevel::Beginner,
            },
            astro_result: astral_llm_domain::AstroCalculationPayload {
                contract_version: "natal_structured_v13".into(),
                chart_type: "natal".into(),
                data: serde_json::json!({ "birth_date": "1990-01-01", "latitude": 48.85 }),
            },
            astrologer_profile: astral_llm_domain::AstrologerProfile {
                profile_id: None,
                name: None,
                tone: astral_llm_domain::ToneProfile::Warm,
                jargon_level: astral_llm_domain::JargonLevel::Beginner,
                wording_style: astral_llm_domain::WordingStyle::Clear,
                preferred_domains: vec![],
                forbidden_wording: vec![],
                custom_instructions: Some("secret note".into()),
            },
            engine: astral_llm_domain::EngineParams {
                provider: None,
                model: None,
                reasoning_effort: None,
                temperature: None,
                max_output_tokens: None,
                domain_count: None,
                allow_fallback: false,
                timeout_ms: None,
                allow_oracle_benchmark: false,
                summary_model: None,
            },
            response_contract: astral_llm_domain::ResponseContract {
                output_schema_version: "natal_reading_v1".into(),
                generation_mode: astral_llm_domain::GenerationMode::SinglePass,
                format: astral_llm_domain::OutputFormat::StructuredJson,
                chapters: vec![],
                global_max_tokens: None,
                include_astro_sources: false,
                include_legal_disclaimer: true,
            },
            safety_policy: None,
        };
        let redacted = redact_request_for_storage(&request);
        assert_eq!(redacted["astro_result"]["data"]["birth_date"], "[REDACTED]");
        assert_eq!(
            redacted["astrologer_profile"]["custom_instructions"],
            "[REDACTED]"
        );
    }

    #[test]
    fn redacts_birth_fields() {
        let input = serde_json::json!({
            "astro_result": {
                "data": { "birth_date": "1990-01-01", "domain_scores": { "identity": 0.5 } }
            }
        });
        let out = redact_value(&input);
        assert_eq!(out["astro_result"]["data"]["birth_date"], "[REDACTED]");
        assert!(out["astro_result"]["data"]["domain_scores"]["identity"].is_number());
    }
}
