use serde_json::Value;

use crate::LlmProviderError;

pub fn parse_response_payload(raw: &str) -> Result<Value, LlmProviderError> {
    serde_json::from_str::<Value>(raw)
        .or_else(|_| {
            let trimmed = raw.trim_start_matches('\u{feff}').trim();
            serde_json::from_str::<Value>(trimmed)
        })
        .or_else(|_| {
            extract_balanced_json_object(raw)
                .ok_or_else(|| serde_json::Error::io(std::io::Error::other("missing json object")))
                .and_then(|json| serde_json::from_str::<Value>(&json))
        })
        .map_err(|err| {
            LlmProviderError::InvalidResponse(format!(
                "provider payload is not valid JSON: {err}"
            ))
        })
}

pub fn parse_model_output_json(raw: &str) -> Option<Value> {
    serde_json::from_str::<Value>(raw)
        .ok()
        .or_else(|| {
            let trimmed = raw.trim();
            let unfenced = trimmed
                .strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .and_then(|value| value.strip_suffix("```"))
                .map(str::trim)
                .unwrap_or(trimmed);
            serde_json::from_str::<Value>(unfenced).ok()
        })
        .or_else(|| extract_balanced_json_object(raw).and_then(|json| serde_json::from_str(&json).ok()))
}

fn extract_balanced_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, ch) in raw[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(raw[start..start + offset + ch.len_utf8()].to_string());
                }
            }
            _ => {}
        }
    }
    None
}
