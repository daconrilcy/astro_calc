use std::fs;
use std::path::{Path, PathBuf};

use astral_llm_infra::AppConfig;
use serde_json::{json, Value};
use sqlx::PgPool;

pub fn contracts_index() -> Value {
    json!({
        "service": "astral_llm_api",
        "contracts": {
            "generate_reading_request_v1": "/v1/schemas/generate_reading_request_v1",
            "generate_reading_response_v1": "/v1/schemas/generate_reading_response_v1",
            "natal_reading_v1": "/v1/schemas/natal_reading_v1",
            "chapter_provider_v1": "/v1/schemas/chapter_provider_v1",
            "summary_provider_v1": "/v1/schemas/summary_provider_v1",
            "integration_job_request_v1": "/v1/schemas/integration_job_request_v1",
            "integration_job_response_v1": "/v1/schemas/integration_job_response_v1",
            "integration_job_status_v1": "/v1/schemas/integration_job_status_v1",
            "integration_service_v1": "/v1/schemas/integration_service_v1",
            "integration_service_contract_v1": "/v1/schemas/integration_service_contract_v1"
            ,
            "horoscope_daily_natal_request": "/v1/schemas/horoscope_daily_natal_request",
            "horoscope_basic_daily_natal_request": "/v1/schemas/horoscope_basic_daily_natal_request",
            "horoscope_premium_daily_local_request": "/v1/schemas/horoscope_premium_daily_local_request",
            "horoscope_period_natal_request": "/v1/schemas/horoscope_period_natal_request",
            "horoscope_period_writer_request": "/v1/schemas/horoscope_period_writer_request",
            "horoscope_period_response": "/v1/schemas/horoscope_period_response",
            "horoscope_response": "/v1/schemas/horoscope_response"
        },
        "openapi": "/openapi.yaml"
    })
}

pub fn load_published_schema(version: &str) -> Option<Value> {
    let filename = match version {
        "generate_reading_request_v1" => "generate_reading_request_v1.schema.json",
        "generate_reading_response_v1" => "generate_reading_response_v1.schema.json",
        "summary_provider_v1" => "summary_provider_v1.schema.json",
        "chapter_provider_v1" => "chapter_provider_v1.schema.json",
        "natal_reading_v1" => "natal_reading_v1.schema.json",
        "integration_job_request_v1" => "integration_job_request_v1.schema.json",
        "integration_job_response_v1" => "integration_job_response_v1.schema.json",
        "integration_job_status_v1" => "integration_job_status_v1.schema.json",
        "integration_service_v1" => "integration_service_v1.schema.json",
        "integration_service_contract_v1" => "integration_service_contract_v1.schema.json",
        "horoscope_daily_natal_request" => "horoscope_daily_natal_request.schema.json",
        "horoscope_basic_daily_natal_request" => {
            "horoscope_basic_daily_natal_request.schema.json"
        }
        "horoscope_premium_daily_local_request" => {
            "horoscope_premium_daily_local_request.schema.json"
        }
        "horoscope_period_natal_request" => "horoscope_period_natal_request.schema.json",
        "horoscope_period_interpretation_request" => {
            "horoscope_period_interpretation_request.schema.json"
        }
        "horoscope_period_writer_request" => "horoscope_period_writer_request.schema.json",
        "horoscope_period_response" => "horoscope_period_response.schema.json",
        "horoscope_response" => "horoscope_response.schema.json",
        "horoscope_interpretation_request" => "horoscope_interpretation_request.schema.json",
        "horoscope_calculation_request" => {
            "../calculator/horoscope_calculation_request.schema.json"
        }
        "horoscope_calculation_response" => {
            "../calculator/horoscope_calculation_response.schema.json"
        }
        "horoscope_period_calculation_request" => {
            "../calculator/horoscope_period_calculation_request.schema.json"
        }
        "horoscope_period_calculation_response" => {
            "../calculator/horoscope_period_calculation_response.schema.json"
        }
        "astro_simplified_natal_request_v1" => {
            "../calculator/astro_simplified_natal_request_v1.schema.json"
        }
        "astro_engine_request_v1" => "../calculator/astro_engine_request_v1.schema.json",
        _ => return None,
    };

    let path = if filename.starts_with("../calculator/") {
        contracts_llm_dir()
            .join("..")
            .join(filename.trim_start_matches("../"))
    } else {
        contracts_llm_dir().join(filename)
    };
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn openapi_bytes() -> Result<Vec<u8>, String> {
    let path = contracts_llm_dir().join("openapi.yaml");
    fs::read(path).map_err(|e| format!("failed to read OpenAPI: {e}"))
}

pub fn contracts_llm_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ASTRAL_LLM_CONTRACTS_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest)
            .join("..")
            .join("..")
            .join("..")
            .join("contracts")
            .join("llm");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("contracts/llm")
}

pub fn service_not_ready(
    message: impl Into<String>,
    details: Value,
) -> (axum::http::StatusCode, axum::Json<Value>) {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(json!({
            "status": "failed",
            "error": {
                "code": "SERVICE_NOT_READY",
                "message": message.into(),
                "details": details
            },
            "request_id": uuid::Uuid::new_v4().to_string()
        })),
    )
}

pub async fn readiness_details(
    config: &AppConfig,
    pool: Option<&PgPool>,
    interpretation_profiles_loaded: usize,
) -> (bool, Value) {
    let prompts_ok = prompts_ready(&config.prompts_dir);
    let mut details = json!({
        "prompts": prompts_ok,
        "interpretation_profiles": interpretation_profiles_loaded > 0,
    });

    let mut ready = prompts_ok && interpretation_profiles_loaded > 0;

    if config.enable_persistence {
        let db_ok = match pool {
            Some(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
            None => false,
        };
        if let Some(obj) = details.as_object_mut() {
            obj.insert("database".to_string(), json!(db_ok));
        }
        ready &= db_ok;
    }

    (ready, details)
}

fn prompts_ready(prompts_dir: &str) -> bool {
    let base = Path::new(prompts_dir).join("natal_prompter").join("v1");
    ["system.md", "task.md", "format.md", "safety.md"]
        .iter()
        .all(|file| base.join(file).is_file())
}
