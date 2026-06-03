use std::collections::HashMap;
use schemars::schema_for;
use jsonschema::JSONSchema;

use astral_llm_domain::{
    generation_response::{ChapterProviderResponse, NatalReadingResponse},
    GenerationError, GenerationErrorCode,
};

pub struct SchemaRegistry {
    schemas: HashMap<String, serde_json::Value>,
    validators: HashMap<String, JSONSchema>,
    provider_schemas: HashMap<String, serde_json::Value>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
            validators: HashMap::new(),
            provider_schemas: HashMap::new(),
        };
        registry.register_natal_reading_v1();
        registry.register_chapter_provider_v1();
        registry
    }

    pub fn get(&self, version: &str) -> Option<&serde_json::Value> {
        self.schemas.get(version)
    }

    pub fn provider_schema(&self, version: &str) -> Option<&serde_json::Value> {
        self.provider_schemas.get(version)
    }

    pub fn validate(&self, version: &str, value: &serde_json::Value) -> Result<(), GenerationError> {
        let validator = self.validators.get(version).ok_or_else(|| {
            GenerationError::new(
                GenerationErrorCode::SchemaValidationFailed,
                format!("unknown schema version: {version}"),
            )
        })?;

        validator.validate(value).map_err(|errors| {
            let details: Vec<String> = errors.map(|e| e.to_string()).collect();
            GenerationError::with_details(
                GenerationErrorCode::SchemaValidationFailed,
                "JSON schema validation failed",
                serde_json::json!({ "errors": details }),
            )
        })
    }

    pub fn validate_chapter(&self, value: &serde_json::Value) -> Result<(), GenerationError> {
        self.validate("chapter_provider_v1", value)
    }

    fn register_natal_reading_v1(&mut self) {
        let schema = schema_for!(NatalReadingResponse);
        let value = serde_json::to_value(schema).expect("schema serializable");
        let provider_schema = strip_schema_for_provider(&value);
        let validator = JSONSchema::compile(&value).expect("valid schema");
        self.schemas
            .insert("natal_reading_v1".to_string(), value);
        self.provider_schemas
            .insert("natal_reading_v1".to_string(), provider_schema);
        self.validators
            .insert("natal_reading_v1".to_string(), validator);
    }

    fn register_chapter_provider_v1(&mut self) {
        let schema = schema_for!(ChapterProviderResponse);
        let value = serde_json::to_value(schema).expect("schema serializable");
        let provider_schema = strip_schema_for_provider(&value);
        let validator = JSONSchema::compile(&value).expect("valid schema");
        self.schemas
            .insert("chapter_provider_v1".to_string(), value);
        self.provider_schemas
            .insert("chapter_provider_v1".to_string(), provider_schema);
        self.validators
            .insert("chapter_provider_v1".to_string(), validator);
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn strip_schema_for_provider(schema: &serde_json::Value) -> serde_json::Value {
    let mut out = schema.clone();
    if let Some(obj) = out.as_object_mut() {
        obj.remove("$schema");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_schema_strips_meta_fields() {
        let registry = SchemaRegistry::new();
        let schema = registry.provider_schema("natal_reading_v1").unwrap();
        assert!(schema.get("$schema").is_none());
    }
}
