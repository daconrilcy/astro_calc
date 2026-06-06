use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use jsonschema::JSONSchema;
use serde_json::Value;

use astral_llm_domain::{
    integration::{IntegrationService, ServiceAvailability},
    GenerationError, GenerationErrorCode,
};

pub struct IntegrationJobValidator {
    envelope_validator: JSONSchema,
    payload_validators: HashMap<String, JSONSchema>,
}

#[derive(Debug, Clone)]
pub struct ValidatedIntegrationJob {
    pub service_code: String,
    pub profile_code: String,
    pub envelope: Value,
    pub payload: Value,
    pub user_language: String,
    pub audience_level: String,
}

impl IntegrationJobValidator {
    pub fn new() -> Self {
        let envelope = load_schema(
            "integration_job_request_v1.schema.json",
            contracts_llm_dir(),
        );
        let envelope_validator =
            JSONSchema::compile(&envelope).expect("integration_job_request_v1 compiles");

        let mut payload_validators = HashMap::new();
        for (contract, dir, filename) in [
            (
                "astro_simplified_natal_request_v1",
                contracts_calculator_dir(),
                "astro_simplified_natal_request_v1.schema.json",
            ),
            (
                "generate_reading_request_v1",
                contracts_llm_dir(),
                "generate_reading_request_v1.schema.json",
            ),
            (
                "astro_engine_request_v1",
                contracts_calculator_dir(),
                "astro_engine_request_v1.schema.json",
            ),
            (
                "horoscope_basic_daily_natal_request_v1",
                contracts_llm_dir(),
                "horoscope_basic_daily_natal_request_v1.schema.json",
            ),
        ] {
            let schema = load_schema(filename, dir);
            if let Ok(validator) = JSONSchema::compile(&schema) {
                payload_validators.insert(contract.to_string(), validator);
            }
        }

        Self {
            envelope_validator,
            payload_validators,
        }
    }

    pub fn validate_envelope(&self, body: &Value) -> Result<Value, GenerationError> {
        self.envelope_validator.validate(body).map_err(|errors| {
            schema_error(
                GenerationErrorCode::InvalidInput,
                "integration job envelope validation failed",
                errors.map(|e| e.to_string()),
            )
        })?;
        Ok(body.clone())
    }

    pub fn validate_service_available(service: &IntegrationService) -> Result<(), GenerationError> {
        if service.availability.is_submittable() {
            return Ok(());
        }
        Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("service not available: {}", service.service_code),
            Value::Null,
        ))
    }

    pub fn validate_payload_contract(
        &self,
        service: &IntegrationService,
        payload: &Value,
    ) -> Result<(), GenerationError> {
        let Some(validator) = self.payload_validators.get(&service.payload_contract) else {
            return Err(GenerationError::with_details(
                GenerationErrorCode::SchemaValidationFailed,
                format!("unknown payload_contract: {}", service.payload_contract),
                Value::Null,
            ));
        };
        validator.validate(payload).map_err(|errors| {
            schema_error(
                GenerationErrorCode::SchemaValidationFailed,
                "payload validation failed",
                errors.map(|e| e.to_string()),
            )
        })
    }

    pub fn validate_from_payload_profile_gate(
        service: &IntegrationService,
        payload: &Value,
    ) -> Result<(), GenerationError> {
        if !service.is_from_payload() {
            return Ok(());
        }
        let profile = payload
            .pointer("/product_context/interpretation_profile_code")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if profile == service.profile_code {
            return Ok(());
        }
        Err(GenerationError::with_details(
            GenerationErrorCode::SchemaValidationFailed,
            format!(
                "interpretation_profile_code must be '{}' for service {}",
                service.profile_code, service.service_code
            ),
            serde_json::json!({
                "expected_profile_code": service.profile_code,
                "received_profile_code": profile,
            }),
        ))
    }

    pub fn validate_job(
        &self,
        body: &Value,
        service: &IntegrationService,
    ) -> Result<ValidatedIntegrationJob, GenerationError> {
        let envelope = self.validate_envelope(body)?;
        let envelope_service_code = envelope
            .get("service_code")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if envelope_service_code != service.service_code {
            return Err(GenerationError::with_details(
                GenerationErrorCode::InvalidInput,
                format!(
                    "service_code mismatch: envelope declares '{envelope_service_code}', catalogue service is '{}'",
                    service.service_code
                ),
                serde_json::json!({
                    "expected_service_code": service.service_code,
                    "received_service_code": envelope_service_code,
                }),
            ));
        }
        Self::validate_service_available(service)?;

        let payload = envelope
            .get("payload")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        self.validate_payload_contract(service, &payload)?;
        Self::validate_from_payload_profile_gate(service, &payload)?;

        Ok(ValidatedIntegrationJob {
            service_code: service.service_code.clone(),
            profile_code: service.profile_code.clone(),
            envelope: envelope.clone(),
            payload,
            user_language: envelope_string(&envelope, "user_language", "fr"),
            audience_level: envelope_string(&envelope, "audience_level", "beginner"),
        })
    }
}

impl Default for IntegrationJobValidator {
    fn default() -> Self {
        Self::new()
    }
}

fn envelope_string(envelope: &Value, key: &str, default: &str) -> String {
    envelope
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
        .to_string()
}

fn schema_error(
    code: GenerationErrorCode,
    message: &str,
    errors: impl Iterator<Item = String>,
) -> GenerationError {
    let details: Vec<String> = errors.collect();
    GenerationError::with_details(code, message, serde_json::json!({ "errors": details }))
}

fn load_schema(filename: &str, dir: PathBuf) -> Value {
    let path = dir.join(filename);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read schema {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("valid schema json")
}

fn contracts_llm_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| resolve_contracts_dir("llm")).clone()
}

fn contracts_calculator_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| resolve_contracts_dir("calculator"))
        .clone()
}

fn resolve_contracts_dir(subdir: &str) -> PathBuf {
    if let Ok(dir) = std::env::var("ASTRAL_LLM_CONTRACTS_DIR") {
        if subdir == "llm" {
            return PathBuf::from(dir);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest)
            .join("..")
            .join("..")
            .join("..")
            .join("contracts")
            .join(subdir);
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("contracts").join(subdir)
}

pub fn service_not_found_error(service_code: &str) -> GenerationError {
    GenerationError::with_details(
        GenerationErrorCode::InvalidInput,
        format!("unknown service_code: {service_code}"),
        serde_json::json!({ "service_code": service_code }),
    )
}

pub fn service_not_listed_error(service: &IntegrationService) -> GenerationError {
    let _ = service;
    GenerationError::with_details(
        GenerationErrorCode::InvalidInput,
        "service is not available for job submission",
        Value::Null,
    )
}

pub fn matches_availability_filter(
    availability: ServiceAvailability,
    include_planned: bool,
) -> bool {
    availability.is_public_listed(include_planned)
}
