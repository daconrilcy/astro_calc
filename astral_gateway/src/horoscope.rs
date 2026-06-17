use std::sync::Arc;

use astral_contracts::{
    ProductTier, QualityMetadataCommon, ResponseMetadataCommon, HOROSCOPE_FREE_DAILY_SERVICE_CODE,
    HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
};
use astral_llm_application::{
    build_calculation_request_for_service, build_interpretation_request,
    build_period_calculation_request_for_service, build_period_writer_request,
    period_editorial_audit, score_calculation, validate_horoscope_response_schema,
    validate_period_public_request, validate_period_response_contract, validate_public_request,
    validate_response_evidence, HoroscopePeriodPublicRequest, HoroscopePublicRequest,
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
        request: HoroscopePublicRequest,
    ) -> Result<HoroscopeReadingResponseV2, GatewayError> {
        let llm_run_id = Uuid::new_v4().to_string();
        let public = validate_public_request(&serde_json::to_value(&request).map_err(|err| {
            GatewayError::bad_request(format!("invalid horoscope request serialization: {err}"))
        })?)
        .map_err(horoscope_bad_request)?;
        let calculation_request = build_calculation_request_for_service(service_code, &public)
            .map_err(horoscope_bad_request)?;
        let calculation = self
            .calculator
            .calculate_horoscope_daily_natal(&calculation_request)
            .await?;
        let signals = score_calculation(&calculation).map_err(horoscope_bad_request)?;
        let mut interpretation = build_interpretation_request(&public, &calculation, &signals)
            .map_err(horoscope_bad_request)?;
        interpretation["debug_run_id"] = json!(llm_run_id);
        let reading = self.llm.render_horoscope_daily(&interpretation).await?;
        validate_horoscope_response_schema(&reading).map_err(horoscope_bad_request)?;
        validate_response_evidence(&interpretation, &reading).map_err(horoscope_bad_request)?;
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
            llm_request: interpretation,
            reading,
            debug: Some(build_horoscope_debug_payload(&llm_run_id, audit, None)),
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
        request: HoroscopePeriodPublicRequest,
    ) -> Result<HoroscopeReadingResponseV2, GatewayError> {
        let llm_run_id = Uuid::new_v4().to_string();
        let public =
            validate_period_public_request(&serde_json::to_value(&request).map_err(|err| {
                GatewayError::bad_request(format!(
                    "invalid horoscope period request serialization: {err}"
                ))
            })?)
            .map_err(horoscope_bad_request)?;
        let calculation_request =
            build_period_calculation_request_for_service(service_code, &public)
                .map_err(horoscope_bad_request)?;
        let calculation = self
            .calculator
            .calculate_horoscope_period_natal(&calculation_request)
            .await?;
        let mut writer_request =
            build_period_writer_request(&public, &calculation).map_err(horoscope_bad_request)?;
        writer_request["debug_run_id"] = json!(llm_run_id);
        let reading = self.llm.render_horoscope_period(&writer_request).await?;
        validate_period_response_contract(&writer_request, &reading)
            .map_err(horoscope_bad_request)?;
        let audit = self.llm.get_run_audit(&llm_run_id).await.ok();

        let mut debug = build_horoscope_debug_payload(
            &llm_run_id,
            audit,
            Some(json!(period_editorial_audit(&writer_request, &reading))),
        );
        if let Some(warning) = public.language_compat_warning.clone() {
            debug["language_compatibility"] = warning;
        }

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
            llm_request: writer_request,
            reading,
            debug: Some(debug),
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
) -> Value {
    let mut debug = json!({
        "run_id": run_id
    });
    if let Some(audit_value) = audit {
        debug["audit"] = audit_value;
    }
    if let Some(editorial_audit) = period_editorial_audit {
        debug["period_editorial_audit"] = editorial_audit;
    }
    debug
}

fn horoscope_bad_request(err: astral_llm_domain::GenerationError) -> GatewayError {
    GatewayError::bad_request(err.detail().message.clone())
}
