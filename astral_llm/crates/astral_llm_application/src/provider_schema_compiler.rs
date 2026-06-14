use std::collections::BTreeMap;

use astral_llm_domain::{
    model_capability::{ModelCapability, StructuredOutputAdapterKind},
    GenerationError, GenerationErrorCode,
};

pub struct ProviderSchemaCompiler;

impl ProviderSchemaCompiler {
    pub fn compile(
        canonical_schema: &serde_json::Value,
        capability: &ModelCapability,
    ) -> Result<serde_json::Value, GenerationError> {
        if !capability.supports_json_schema_strict {
            return Err(GenerationError::new(
                GenerationErrorCode::UnsupportedCapability,
                "provider model does not support strict JSON schema",
            ));
        }

        let schema = match capability.structured_output_adapter {
            StructuredOutputAdapterKind::OpenAiResponsesTextFormat
            | StructuredOutputAdapterKind::MistralResponseFormatJsonSchema => {
                prepare_strict_json_schema(canonical_schema)
            }
            StructuredOutputAdapterKind::AnthropicOutputConfigFormat
            | StructuredOutputAdapterKind::PromptOnly => {
                let mut schema = canonical_schema.clone();
                strip_provider_meta_fields(&mut schema);
                schema
            }
            StructuredOutputAdapterKind::MistralResponseFormatJsonObject => {
                return Err(GenerationError::new(
                    GenerationErrorCode::UnsupportedCapability,
                    "strict output required but model only supports json_object mode",
                ));
            }
        };

        match capability.structured_output_adapter {
            StructuredOutputAdapterKind::OpenAiResponsesTextFormat
            | StructuredOutputAdapterKind::AnthropicOutputConfigFormat
            | StructuredOutputAdapterKind::PromptOnly => Ok(schema),
            StructuredOutputAdapterKind::MistralResponseFormatJsonSchema => {
                Ok(wrap_mistral_schema(schema))
            }
            StructuredOutputAdapterKind::MistralResponseFormatJsonObject => unreachable!(),
        }
    }
}

pub(crate) fn prepare_strict_json_schema(schema: &serde_json::Value) -> serde_json::Value {
    let defs = collect_definitions(schema);
    let mut out = schema.clone();
    inline_refs(&mut out, &defs);
    strip_provider_meta_fields(&mut out);
    enforce_strict_object_rules(&mut out);
    out
}

/// Verrouille `code` sur la valeur attendue (evite les suffixes type `emotional_life_natal_premium_v1`).
pub fn pin_chapter_code(schema: &mut serde_json::Value, chapter_code: &str) {
    let Some(props) = schema.pointer_mut("/properties") else {
        return;
    };
    let Some(obj) = props.as_object_mut() else {
        return;
    };
    obj.insert(
        "code".to_string(),
        serde_json::json!({
            "type": "string",
            "const": chapter_code
        }),
    );
}

fn wrap_mistral_schema(schema: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "structured_reading",
            "schema": schema,
            "strict": true
        }
    })
}

fn collect_definitions(schema: &serde_json::Value) -> BTreeMap<String, serde_json::Value> {
    let mut defs = BTreeMap::new();
    let Some(obj) = schema.as_object() else {
        return defs;
    };
    if let Some(local_defs) = obj.get("definitions").and_then(|v| v.as_object()) {
        defs.extend(local_defs.clone());
    }
    if let Some(local_defs) = obj.get("$defs").and_then(|v| v.as_object()) {
        defs.extend(local_defs.clone());
    }
    defs
}

fn definition_name_from_ref(reference: &str) -> Option<&str> {
    reference
        .strip_prefix("#/definitions/")
        .or_else(|| reference.strip_prefix("#/$defs/"))
}

fn inline_refs(value: &mut serde_json::Value, defs: &BTreeMap<String, serde_json::Value>) {
    match value {
        serde_json::Value::Object(obj) => {
            if let Some(reference) = obj.get("$ref").and_then(|v| v.as_str()) {
                if let Some(name) = definition_name_from_ref(reference) {
                    if let Some(def) = defs.get(name) {
                        *value = def.clone();
                        inline_refs(value, defs);
                        return;
                    }
                }
            }

            for key in [
                "properties",
                "items",
                "additionalProperties",
                "patternProperties",
            ] {
                if let Some(child) = obj.get_mut(key) {
                    if key == "properties" {
                        if let Some(props) = child.as_object_mut() {
                            for prop in props.values_mut() {
                                inline_refs(prop, defs);
                            }
                        }
                    } else {
                        inline_refs(child, defs);
                    }
                }
            }

            for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                if let Some(items) = obj.get_mut(key).and_then(|v| v.as_array_mut()) {
                    for item in items.iter_mut() {
                        inline_refs(item, defs);
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                inline_refs(item, defs);
            }
        }
        _ => {}
    }
}

fn strip_provider_meta_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(obj) => {
            obj.remove("$schema");
            obj.remove("$id");
            obj.remove("$defs");
            obj.remove("definitions");
            obj.remove("title");
            obj.remove("description");

            for key in [
                "properties",
                "items",
                "additionalProperties",
                "patternProperties",
            ] {
                if let Some(child) = obj.get_mut(key) {
                    if key == "properties" {
                        if let Some(props) = child.as_object_mut() {
                            for prop in props.values_mut() {
                                strip_provider_meta_fields(prop);
                            }
                        }
                    } else {
                        strip_provider_meta_fields(child);
                    }
                }
            }

            for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                if let Some(items) = obj.get_mut(key).and_then(|v| v.as_array_mut()) {
                    for item in items.iter_mut() {
                        strip_provider_meta_fields(item);
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                strip_provider_meta_fields(item);
            }
        }
        _ => {}
    }
}

fn enforce_strict_object_rules(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(obj) => {
            if !obj.contains_key("type") {
                if let Some(const_value) = obj.get("const") {
                    let inferred = match const_value {
                        serde_json::Value::String(_) => Some("string"),
                        serde_json::Value::Number(_) => Some("number"),
                        serde_json::Value::Bool(_) => Some("boolean"),
                        serde_json::Value::Array(_) => Some("array"),
                        serde_json::Value::Object(_) => Some("object"),
                        serde_json::Value::Null => Some("null"),
                    };
                    if let Some(inferred) = inferred {
                        obj.insert("type".into(), serde_json::json!(inferred));
                    }
                }
            }

            let is_object = obj.get("type").and_then(|t| t.as_str()) == Some("object")
                || (obj.contains_key("properties") && !obj.contains_key("$ref"));

            if is_object {
                obj.insert(
                    "additionalProperties".into(),
                    serde_json::Value::Bool(false),
                );
                if let Some(props) = obj.get("properties").and_then(|p| p.as_object()) {
                    let required: Vec<&str> = props.keys().map(String::as_str).collect();
                    obj.insert("required".into(), serde_json::json!(required));
                }
            }

            for key in [
                "properties",
                "items",
                "additionalProperties",
                "patternProperties",
            ] {
                if let Some(child) = obj.get_mut(key) {
                    if key == "properties" {
                        if let Some(props) = child.as_object_mut() {
                            for prop in props.values_mut() {
                                enforce_strict_object_rules(prop);
                            }
                        }
                    } else {
                        enforce_strict_object_rules(child);
                    }
                }
            }

            for key in ["allOf", "anyOf", "oneOf", "prefixItems"] {
                if let Some(items) = obj.get_mut(key).and_then(|v| v.as_array_mut()) {
                    for item in items.iter_mut() {
                        enforce_strict_object_rules(item);
                    }
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                enforce_strict_object_rules(item);
            }
        }
        _ => {}
    }
}
