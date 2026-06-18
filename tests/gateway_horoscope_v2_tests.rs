use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use astral_gateway::{
    horoscope::{GenerateHoroscopeDailyReadingUseCase, GenerateHoroscopePeriodReadingUseCase},
    ports::{CalculatorPort, LlmPort},
};
use async_trait::async_trait;
use serde_json::{json, Value};

struct FixtureCalculator {
    daily: Value,
    period: Value,
}

#[async_trait]
impl CalculatorPort for FixtureCalculator {
    async fn calculate_simplified_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        unreachable!()
    }

    async fn calculate_full_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        unreachable!()
    }

    async fn calculate_horoscope_daily_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.daily.clone())
    }

    async fn calculate_horoscope_period_natal(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.period.clone())
    }
}

struct FixtureLlm {
    daily: Value,
    period: Value,
    captured_daily_request: Mutex<Option<Value>>,
    captured_period_request: Mutex<Option<Value>>,
}

#[async_trait]
impl LlmPort for FixtureLlm {
    async fn generate_reading(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        unreachable!()
    }

    async fn build_horoscope_daily_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(serde_json::json!({
            "contract_version": "horoscope_calculation_request",
            "service_code": request.get("service_code").cloned().unwrap_or(Value::Null),
            "period": {
                "date": request.pointer("/public_request/date").cloned().unwrap_or(Value::Null),
                "timezone": request.pointer("/public_request/timezone").cloned().unwrap_or(Value::Null)
            },
            "chart_calculation_id": request.pointer("/public_request/chart_calculation_id").cloned().unwrap_or(Value::Null),
            "slots": []
        }))
    }

    async fn build_horoscope_period_calculation_request(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(serde_json::json!({
            "contract_version": "horoscope_period_calculation_request",
            "service_code": request.get("service_code").cloned().unwrap_or(Value::Null),
            "chart_calculation_id": request.pointer("/public_request/chart_calculation_id").cloned().unwrap_or(Value::Null),
            "period_resolution": {},
            "scan_plan": { "snapshots": [] }
        }))
    }

    async fn render_horoscope_daily_gateway(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        *self
            .captured_daily_request
            .lock()
            .expect("daily request lock") = Some(request.clone());
        Ok(json!({
            "llm_request": request,
            "reading": self.daily
        }))
    }

    async fn render_horoscope_period_gateway(
        &self,
        request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        *self
            .captured_period_request
            .lock()
            .expect("period request lock") = Some(request.clone());
        Ok(json!({
            "llm_request": request,
            "reading": self.period,
            "period_editorial_audit": {
                "warnings": []
            }
        }))
    }

    async fn get_run_audit(
        &self,
        run_id: &str,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(serde_json::json!({
            "run_id": run_id,
            "token_usage": {
                "summary": {
                    "input_tokens": 42,
                    "output_tokens": 314,
                    "cache_tokens": 12,
                    "reasoning_tokens": 8
                },
                "cost": {
                    "currency": "USD",
                    "estimated_total": 0.001234
                }
            },
            "steps": [
                {
                    "step_type": "writer",
                    "status": "completed",
                    "provider": "openai",
                    "model": "gpt-5-mini",
                    "latency_ms": 1200,
                    "token_usage": {
                        "summary": {
                            "input_tokens": 42,
                            "output_tokens": 314,
                            "cache_tokens": 12,
                            "reasoning_tokens": 8
                        }
                    }
                }
            ]
        }))
    }
}

#[tokio::test]
async fn gateway_daily_basic_uses_calculator_then_llm() {
    let calculator = Arc::new(FixtureCalculator {
        daily: read_golden("horoscope_calculation_response_basic_daily_paris_1990.json"),
        period: Value::Null,
    });
    let llm = Arc::new(FixtureLlm {
        daily: read_golden("horoscope_response_basic_daily_fake.json"),
        period: Value::Null,
        captured_daily_request: Mutex::new(None),
        captured_period_request: Mutex::new(None),
    });
    let use_case = GenerateHoroscopeDailyReadingUseCase::new(calculator, llm.clone());

    let response = use_case
        .execute(
            astral_contracts::HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            json!({
                "date": "1990-06-15",
                "timezone": "Europe/Paris",
                "target_language": "fr",
                "chart_calculation_id": "123",
                "audience_level": "general"
            }),
        )
        .await
        .expect("daily response");

    assert_eq!(response.metadata.variant, "daily");
    assert_eq!(
        response.metadata.product_code,
        astral_contracts::HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
    );
    let debug_run_id = response
        .debug
        .as_ref()
        .and_then(|value| value.get("run_id"))
        .and_then(Value::as_str)
        .expect("debug run id");
    uuid::Uuid::parse_str(debug_run_id).expect("valid uuid");
    let captured_daily_request = llm
        .captured_daily_request
        .lock()
        .expect("daily request lock");
    let captured_run_id = captured_daily_request
        .as_ref()
        .and_then(|value| value.get("debug_run_id"))
        .and_then(Value::as_str)
        .expect("captured debug_run_id");
    assert_eq!(captured_run_id, debug_run_id);
    let embedded_audit = response
        .debug
        .as_ref()
        .and_then(|value| value.get("audit"))
        .and_then(Value::as_object)
        .expect("embedded audit");
    assert_eq!(
        embedded_audit.get("run_id").and_then(Value::as_str),
        Some(debug_run_id)
    );
}

#[tokio::test]
async fn gateway_period_free_uses_calculator_then_llm() {
    let calculator = Arc::new(FixtureCalculator {
        daily: Value::Null,
        period: read_golden(
            "horoscope_period_calculation_response_free_next_7_days_paris_1990.json",
        ),
    });
    let llm = Arc::new(FixtureLlm {
        daily: Value::Null,
        period: read_golden("horoscope_period_response_free_next_7_days_fake.json"),
        captured_daily_request: Mutex::new(None),
        captured_period_request: Mutex::new(None),
    });
    let use_case = GenerateHoroscopePeriodReadingUseCase::new(calculator, llm.clone());

    let response = use_case
        .execute(
            astral_contracts::HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            json!({
                "anchor_date": "1990-06-15",
                "timezone": "Europe/Paris",
                "target_language": "fr",
                "chart_calculation_id": "123",
                "audience_level": "general"
            }),
        )
        .await
        .expect("period response");

    assert_eq!(response.metadata.variant, "period");
    assert_eq!(
        response.metadata.product_code,
        astral_contracts::HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
    );
    let debug_run_id = response
        .debug
        .as_ref()
        .and_then(|value| value.get("run_id"))
        .and_then(Value::as_str)
        .expect("debug run id");
    uuid::Uuid::parse_str(debug_run_id).expect("valid uuid");
    let captured_period_request = llm
        .captured_period_request
        .lock()
        .expect("period request lock");
    let captured_run_id = captured_period_request
        .as_ref()
        .and_then(|value| value.get("debug_run_id"))
        .and_then(Value::as_str)
        .expect("captured debug_run_id");
    assert_eq!(captured_run_id, debug_run_id);
    let embedded_audit = response
        .debug
        .as_ref()
        .and_then(|value| value.get("audit"))
        .and_then(Value::as_object)
        .expect("embedded audit");
    assert_eq!(
        embedded_audit.get("run_id").and_then(Value::as_str),
        Some(debug_run_id)
    );
}

fn read_golden(name: &str) -> Value {
    let path = repo_root().join("tests").join("golden").join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("read golden")).expect("json")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root")
}
