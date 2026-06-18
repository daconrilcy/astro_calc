use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::JSONSchema;
use serde_json::Value;

use crate::config::validate_path_within;

pub struct SchemaRegistry {
    schemas: HashMap<String, Value>,
    validators: HashMap<String, JSONSchema>,
}

impl SchemaRegistry {
    pub fn from_dir(dir: &Path) -> Result<Self, String> {
        let mut registry = Self {
            schemas: HashMap::new(),
            validators: HashMap::new(),
        };

        let mappings = [
            (
                "astro_engine_request_v1",
                "astro_engine_request_v1.schema.json",
            ),
            (
                "astro_engine_response_v1",
                "astro_engine_response_v1.schema.json",
            ),
            ("natal_structured_v13", "natal_structured_v13.schema.json"),
            (
                "llm_projection_natal_v1",
                "llm_projection_natal_v1.schema.json",
            ),
            (
                "astro_simplified_natal_request_v1",
                "astro_simplified_natal_request_v1.schema.json",
            ),
            (
                "astro_simplified_natal_response_v1",
                "astro_simplified_natal_response_v1.schema.json",
            ),
            (
                "natal_simplified_structured_v1",
                "natal_simplified_structured_v1.schema.json",
            ),
            (
                "llm_projection_natal_simplified_v1",
                "llm_projection_natal_simplified_v1.schema.json",
            ),
            (
                "horoscope_calculation_request",
                "horoscope_calculation_request.schema.json",
            ),
            (
                "horoscope_calculation_response",
                "horoscope_calculation_response.schema.json",
            ),
            (
                "horoscope_period_calculation_request",
                "horoscope_period_calculation_request.schema.json",
            ),
            (
                "horoscope_period_calculation_response",
                "horoscope_period_calculation_response.schema.json",
            ),
        ];

        for (version, filename) in mappings {
            let path = dir.join(filename);
            let raw = fs::read_to_string(&path)
                .map_err(|e| format!("failed to read schema {}: {e}", path.display()))?;
            let value: Value =
                serde_json::from_str(&raw).map_err(|e| format!("invalid schema JSON: {e}"))?;
            let validator = JSONSchema::options()
                .compile(&value)
                .map_err(|e| format!("schema compile failed for {version}: {e}"))?;
            registry.schemas.insert(version.to_string(), value);
            registry.validators.insert(version.to_string(), validator);
        }

        Ok(registry)
    }

    pub fn get(&self, version: &str) -> Option<&Value> {
        self.schemas.get(version)
    }

    pub fn validate(&self, version: &str, payload: &Value) -> Result<(), Vec<String>> {
        let validator = self
            .validators
            .get(version)
            .ok_or_else(|| vec![format!("unknown schema version: {version}")])?;

        validator
            .validate(payload)
            .err()
            .map(|errors| errors.map(|e| e.to_string()).collect())
            .map_or(Ok(()), Err)
    }

    pub fn contract_links(&self) -> HashMap<String, String> {
        self.schemas
            .keys()
            .map(|version| (version.clone(), format!("/v1/schemas/{version}")))
            .collect()
    }
}

pub fn openapi_bytes(path: &Path, allowed_dir: &Path) -> Result<Vec<u8>, String> {
    validate_path_within(path, allowed_dir)?;
    if !path.is_file() {
        return Err(format!("OpenAPI file not found: {}", path.display()));
    }
    fs::read(path).map_err(|e| format!("failed to read OpenAPI at {}: {e}", path.display()))
}

pub fn repo_contracts_calculator_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("contracts")
        .join("calculator")
}
