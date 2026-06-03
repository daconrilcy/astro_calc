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

fn prepare_strict_json_schema(schema: &serde_json::Value) -> serde_json::Value {
    let defs = collect_definitions(schema);
    let mut out = schema.clone();
    inline_refs(&mut out, &defs);
    strip_provider_meta_fields(&mut out);
    enforce_strict_object_rules(&mut out);
    out
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
            let is_object = obj.get("type").and_then(|t| t.as_str()) == Some("object")
                || (obj.contains_key("properties") && !obj.contains_key("$ref"));

            if is_object {
                obj.insert("additionalProperties".into(), serde_json::Value::Bool(false));
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

#[cfg(test)]
mod tests {
    use super::*;
    use astral_llm_domain::provider::{ProviderKind, StructuredOutputMode};
    use astral_llm_domain::ModelCapability;

    fn openai_cap() -> ModelCapability {
        ModelCapability {
            provider: ProviderKind::OpenAi,
            model: "gpt-4.1".into(),
            supports_json_schema_strict: true,
            supports_json_object: true,
            supports_reasoning_effort: true,
            supports_streaming: true,
            supports_native_safety_prompt: false,
            max_input_tokens: 128_000,
            max_output_tokens: 16_384,
            structured_output_mode: StructuredOutputMode::JsonSchemaStrict,
            structured_output_adapter: StructuredOutputAdapterKind::OpenAiResponsesTextFormat,
            storage_disable_supported: true,
            is_active: true,
        }
    }

    #[test]
    fn strips_defs_for_openai() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": { "title": { "type": "string" } }
        });
        let compiled = ProviderSchemaCompiler::compile(&schema, &openai_cap()).unwrap();
        assert!(compiled.get("$schema").is_none());
        assert_eq!(compiled["additionalProperties"], false);
        assert_eq!(
            compiled["required"],
            serde_json::json!(["title"])
        );
    }

    #[test]
    fn openai_schema_sets_additional_properties_false() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "meta": {
                    "type": "object",
                    "properties": { "count": { "type": "integer" } },
                    "required": ["count"]
                }
            },
            "required": ["title", "meta"]
        });
        let compiled = ProviderSchemaCompiler::compile(&schema, &openai_cap()).unwrap();
        assert_eq!(compiled["additionalProperties"], false);
        assert_eq!(compiled["properties"]["meta"]["additionalProperties"], false);
    }

    #[test]
    fn inlines_chapter_schema_refs_for_openai() {
        use crate::schema_registry::SchemaRegistry;

        let registry = SchemaRegistry::new();
        let schema = registry.get("chapter_provider_v1").unwrap();
        let compiled = ProviderSchemaCompiler::compile(schema, &openai_cap()).unwrap();

        assert_eq!(compiled["additionalProperties"], false);
        assert!(compiled.get("definitions").is_none());
        assert!(compiled["properties"]["confidence"].get("$ref").is_none());
        assert_eq!(
            compiled["properties"]["confidence"]["enum"],
            serde_json::json!(["low", "medium", "high"])
        );
        assert_eq!(
            compiled["properties"]["astro_basis"]["items"]["additionalProperties"],
            false
        );
        assert!(compiled["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "astro_basis"));
    }
}
