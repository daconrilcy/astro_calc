use std::sync::Arc;

use astral_llm_domain::{
    generation_response::NatalReadingResponse, GenerationError, GenerationErrorCode,
};

use crate::schema_registry::SchemaRegistry;
use crate::token_budget::TokenBudget;

pub struct ResponseValidator {
    schema_registry: Arc<SchemaRegistry>,
}

impl ResponseValidator {
    pub fn new(schema_registry: Arc<SchemaRegistry>) -> Self {
        Self { schema_registry }
    }

    pub fn schema_registry(&self) -> &SchemaRegistry {
        &self.schema_registry
    }

    pub fn validate_chapter(&self, value: &serde_json::Value) -> Result<(), GenerationError> {
        self.schema_registry.validate("chapter_provider_v1", value)
    }

    pub fn validate_and_parse(
        &self,
        schema_version: &str,
        raw: &serde_json::Value,
        chapter_contracts: &[astral_llm_domain::ChapterContract],
    ) -> Result<NatalReadingResponse, GenerationError> {
        self.schema_registry.validate(schema_version, raw)?;

        let reading: NatalReadingResponse = serde_json::from_value(raw.clone()).map_err(|e| {
            GenerationError::new(
                GenerationErrorCode::InvalidJsonOutput,
                format!("deserialization failed: {e}"),
            )
        })?;

        if !chapter_contracts.is_empty() {
            let pairs: Vec<(String, String)> = reading
                .chapters
                .iter()
                .map(|c| (c.code.clone(), c.body.clone()))
                .collect();
            TokenBudget::validate_chapter_lengths(&pairs, chapter_contracts)?;
        }

        Ok(reading)
    }
}

impl Default for ResponseValidator {
    fn default() -> Self {
        Self::new(Arc::new(SchemaRegistry::new()))
    }
}
