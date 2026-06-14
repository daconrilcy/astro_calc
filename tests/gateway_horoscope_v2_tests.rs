use std::{fs, path::PathBuf, sync::Arc};

use astral_gateway::{
    horoscope::{GenerateHoroscopeDailyReadingUseCase, GenerateHoroscopePeriodReadingUseCase},
    ports::{CalculatorPort, LlmPort},
};
use astral_llm_application::{HoroscopePeriodPublicRequest, HoroscopePublicRequest};
use astral_llm_domain::{GenerateReadingRequest, GenerateReadingResponse};
use async_trait::async_trait;
use serde_json::Value;

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
}

#[async_trait]
impl LlmPort for FixtureLlm {
    async fn generate_reading(
        &self,
        _request: &GenerateReadingRequest,
    ) -> Result<GenerateReadingResponse, astral_gateway::error::GatewayError> {
        unreachable!()
    }

    async fn render_horoscope_daily(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.daily.clone())
    }

    async fn render_horoscope_period(
        &self,
        _request: &Value,
    ) -> Result<Value, astral_gateway::error::GatewayError> {
        Ok(self.period.clone())
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
    });
    let use_case = GenerateHoroscopeDailyReadingUseCase::new(calculator, llm);

    let response = use_case
        .execute(
            astral_contracts::HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE,
            HoroscopePublicRequest {
                date: "1990-06-15".into(),
                timezone: "Europe/Paris".into(),
                target_language: "fr".into(),
                chart_calculation_id: "123".into(),
                location: None,
                audience_level: "general".into(),
                detail_level: None,
            },
        )
        .await
        .expect("daily response");

    assert_eq!(response.metadata.variant, "daily");
    assert_eq!(
        response.metadata.product_code,
        astral_contracts::HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
    );
}

#[tokio::test]
async fn gateway_period_free_uses_calculator_then_llm() {
    let calculator = Arc::new(FixtureCalculator {
        daily: Value::Null,
        period: read_golden("horoscope_period_calculation_response_free_next_7_days_paris_1990.json"),
    });
    let llm = Arc::new(FixtureLlm {
        daily: Value::Null,
        period: read_golden("horoscope_period_response_free_next_7_days_fake.json"),
    });
    let use_case = GenerateHoroscopePeriodReadingUseCase::new(calculator, llm);

    let response = use_case
        .execute(
            astral_contracts::HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
            HoroscopePeriodPublicRequest {
                anchor_date: "1990-06-15".into(),
                timezone: "Europe/Paris".into(),
                target_language: "fr".into(),
                target_language_code: None,
                chart_calculation_id: "123".into(),
                audience_level: "general".into(),
                astrologer_persona: None,
                language_compat_warning: None,
            },
        )
        .await
        .expect("period response");

    assert_eq!(response.metadata.variant, "period");
    assert_eq!(
        response.metadata.product_code,
        astral_contracts::HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
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
