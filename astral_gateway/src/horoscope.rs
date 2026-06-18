use std::sync::Arc;

use astral_contracts::{
    ProductTier, QualityMetadataCommon, ResponseMetadataCommon, HOROSCOPE_FREE_DAILY_SERVICE_CODE,
    HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    error::GatewayError,
    ports::{CalculatorPort, LlmPort},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoroscopeReadingResponseV2 {
    pub metadata: ResponseMetadataCommon,
    pub quality: QualityMetadataCommon,
    pub calculation: Value,
    pub llm_request: Value,
    pub reading: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<Value>,
}

pub struct GenerateHoroscopeDailyReadingUseCase {
    calculator: Arc<dyn CalculatorPort>,
    llm: Arc<dyn LlmPort>,
}

pub struct GenerateHoroscopePeriodReadingUseCase {
    calculator: Arc<dyn CalculatorPort>,
    llm: Arc<dyn LlmPort>,
}

impl GenerateHoroscopeDailyReadingUseCase {
    pub fn new(calculator: Arc<dyn CalculatorPort>, llm: Arc<dyn LlmPort>) -> Self {
        Self { calculator, llm }
    }

    pub async fn execute(
        &self,
        service_code: &str,
        request: Value,
    ) -> Result<HoroscopeReadingResponseV2, GatewayError> {
        let llm_run_id = Uuid::new_v4().to_string();
        let calculation_request = self
            .llm
            .build_horoscope_daily_calculation_request(&json!({
                "service_code": service_code,
                "public_request": request.clone()
            }))
            .await?;
        let calculation = self
            .calculator
            .calculate_horoscope_daily_natal(&calculation_request)
            .await?;
        let rendered = self
            .llm
            .render_horoscope_daily_gateway(&json!({
                "service_code": service_code,
                "public_request": request,
                "calculation": calculation,
                "debug_run_id": llm_run_id
            }))
            .await?;
        let llm_request = rendered.get("llm_request").cloned().unwrap_or(Value::Null);
        let reading = rendered.get("reading").cloned().unwrap_or(Value::Null);
        let audit = self.llm.get_run_audit(&llm_run_id).await.ok();

        Ok(HoroscopeReadingResponseV2 {
            metadata: ResponseMetadataCommon {
                product_code: service_code.to_string(),
                tier: daily_tier(service_code),
                variant: "daily".to_string(),
                contract_version: "horoscope_reading_response_v2".to_string(),
            },
            quality: QualityMetadataCommon {
                calculator_contract_version: Some("horoscope_calculation_response".to_string()),
                llm_contract_version: Some("horoscope_response".to_string()),
                reading_completeness: None,
            },
            calculation,
            llm_request,
            reading,
            debug: Some(build_horoscope_debug_payload(
                &llm_run_id,
                audit,
                None,
                None,
            )),
        })
    }
}

impl GenerateHoroscopePeriodReadingUseCase {
    pub fn new(calculator: Arc<dyn CalculatorPort>, llm: Arc<dyn LlmPort>) -> Self {
        Self { calculator, llm }
    }

    pub async fn execute(
        &self,
        service_code: &str,
        request: Value,
    ) -> Result<HoroscopeReadingResponseV2, GatewayError> {
        let llm_run_id = Uuid::new_v4().to_string();
        let calculation_request = self
            .llm
            .build_horoscope_period_calculation_request(&json!({
                "service_code": service_code,
                "public_request": request.clone()
            }))
            .await?;
        let calculation = self
            .calculator
            .calculate_horoscope_period_natal(&calculation_request)
            .await?;
        let rendered = self
            .llm
            .render_horoscope_period_gateway(&json!({
                "service_code": service_code,
                "public_request": request,
                "calculation": calculation,
                "debug_run_id": llm_run_id
            }))
            .await?;
        let llm_request = rendered.get("llm_request").cloned().unwrap_or(Value::Null);
        let reading = rendered.get("reading").cloned().unwrap_or(Value::Null);
        let period_editorial_audit = rendered.get("period_editorial_audit").cloned();
        let language_compatibility = rendered.get("language_compatibility").cloned();
        let audit = self.llm.get_run_audit(&llm_run_id).await.ok();

        Ok(HoroscopeReadingResponseV2 {
            metadata: ResponseMetadataCommon {
                product_code: service_code.to_string(),
                tier: period_tier(service_code),
                variant: "period".to_string(),
                contract_version: "horoscope_reading_response_v2".to_string(),
            },
            quality: QualityMetadataCommon {
                calculator_contract_version: Some(
                    "horoscope_period_calculation_response".to_string(),
                ),
                llm_contract_version: Some("horoscope_period_response".to_string()),
                reading_completeness: None,
            },
            calculation,
            llm_request,
            reading,
            debug: Some(build_horoscope_debug_payload(
                &llm_run_id,
                audit,
                period_editorial_audit,
                language_compatibility,
            )),
        })
    }
}

fn daily_tier(service_code: &str) -> ProductTier {
    match service_code {
        HOROSCOPE_FREE_DAILY_SERVICE_CODE => ProductTier::Free,
        HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE => ProductTier::Premium,
        _ => ProductTier::Basic,
    }
}

fn period_tier(service_code: &str) -> ProductTier {
    match service_code {
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE => ProductTier::Free,
        HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE => ProductTier::Premium,
        _ => ProductTier::Basic,
    }
}

fn build_horoscope_debug_payload(
    run_id: &str,
    audit: Option<Value>,
    period_editorial_audit: Option<Value>,
    language_compatibility: Option<Value>,
) -> Value {
    let mut debug = json!({ "run_id": run_id });
    if let Some(audit_value) = audit {
        debug["audit"] = audit_value;
    }
    if let Some(editorial_audit) = period_editorial_audit {
        debug["period_editorial_audit"] = editorial_audit;
    }
    if let Some(warning) = language_compatibility {
        debug["language_compatibility"] = warning;
    }
    debug
}
