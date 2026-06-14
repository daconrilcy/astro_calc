use std::sync::Arc;

use astral_contracts::{
    NatalProductCode, NatalVariant, ProductTier, QualityMetadataCommon, ResponseMetadataCommon,
};
use astral_llm_application::{
    build_reading_request, build_reading_request_from_engine, validate_engine_response,
    validate_simplified_calculation_request, SIMPLIFIED_PROFILE,
};
use astral_llm_domain::{
    generation_request::AudienceLevel, GenerateReadingRequest,
};
use serde_json::Value;

use crate::{
    contracts::{NatalReadingRequestV2, NatalReadingResponseV2},
    error::GatewayError,
    ports::{CalculatorPort, LlmPort},
};

#[derive(Debug, Clone, Copy)]
pub struct NatalGatewayPolicy {
    pub variant: NatalVariant,
    pub tier: ProductTier,
}

impl NatalGatewayPolicy {
    pub fn product_code(&self) -> NatalProductCode {
        NatalProductCode::from_parts(self.variant, self.tier)
    }

    pub fn default_audience_level(&self) -> AudienceLevel {
        match self.tier {
            ProductTier::Free => AudienceLevel::Beginner,
            ProductTier::Basic => AudienceLevel::Intermediate,
            ProductTier::Premium => AudienceLevel::Expert,
        }
    }

    pub fn projection_level(&self) -> &'static str {
        match self.tier {
            ProductTier::Free => "compact",
            ProductTier::Basic => "standard",
            ProductTier::Premium => "rich",
        }
    }

    pub fn interpretation_profile_code(&self) -> &'static str {
        match self.variant {
            NatalVariant::Simplified => SIMPLIFIED_PROFILE,
            NatalVariant::Full => match self.tier {
                ProductTier::Free => "natal_light",
                ProductTier::Basic => "natal_basic",
                ProductTier::Premium => "natal_premium",
            },
        }
    }
}

pub struct GenerateNatalReadingUseCase {
    calculator: Arc<dyn CalculatorPort>,
    llm: Arc<dyn LlmPort>,
}

impl GenerateNatalReadingUseCase {
    pub fn new(calculator: Arc<dyn CalculatorPort>, llm: Arc<dyn LlmPort>) -> Self {
        Self { calculator, llm }
    }

    pub async fn execute(
        &self,
        policy: NatalGatewayPolicy,
        request: NatalReadingRequestV2,
    ) -> Result<NatalReadingResponseV2, GatewayError> {
        let calculation = match policy.variant {
            NatalVariant::Simplified => {
                let calculation_request = simplified_calculation_request(&request)?;
                validate_simplified_calculation_request(&calculation_request)
                    .map_err(|err| GatewayError::bad_request(err.detail().message.clone()))?;
                self.calculator
                    .calculate_simplified_natal(&calculation_request)
                    .await?
            }
            NatalVariant::Full => {
                let calculation_request = full_calculation_request(&request, &policy)?;
                self.calculator.calculate_full_natal(&calculation_request).await?
            }
        };

        let reading_request = build_llm_request(&request, &calculation, &policy)?;
        let reading = self.llm.generate_reading(&reading_request).await?;

        Ok(NatalReadingResponseV2 {
            metadata: ResponseMetadataCommon {
                product_code: policy.product_code().as_str().to_string(),
                tier: policy.tier,
                variant: policy.variant.as_str().to_string(),
                contract_version: "natal_reading_response_v2".to_string(),
            },
            quality: QualityMetadataCommon {
                calculator_contract_version: calculation_contract_version(&calculation),
                llm_contract_version: Some("generate_reading_response_v1".to_string()),
                reading_completeness: reading_completeness_hint(&calculation),
            },
            calculation: Some(calculation),
            reading,
        })
    }
}

fn simplified_calculation_request(request: &NatalReadingRequestV2) -> Result<Value, GatewayError> {
    let payload = serde_json::json!({
        "request_contract_version": "astro_simplified_natal_request_v1",
        "request_id": request.context.request_id,
        "birth": {
            "date": request.birth.date,
            "time": request.birth.time,
            "timezone": request.birth.timezone,
            "location": request.birth.location,
        },
        "calculation": {
            "zodiacal_reference_system": "tropical",
            "house_system": "placidus"
        }
    });
    Ok(payload)
}

fn full_calculation_request(
    request: &NatalReadingRequestV2,
    policy: &NatalGatewayPolicy,
) -> Result<Value, GatewayError> {
    let time = request
        .birth
        .time
        .clone()
        .ok_or_else(|| GatewayError::bad_request("birth.time is required for full natal"))?;
    let timezone = request
        .birth
        .timezone
        .clone()
        .ok_or_else(|| GatewayError::bad_request("birth.timezone is required for full natal"))?;
    let location = request
        .birth
        .location
        .clone()
        .ok_or_else(|| GatewayError::bad_request("birth.location is required for full natal"))?;

    Ok(serde_json::json!({
        "request_contract_version": "astro_engine_request_v1",
        "request_id": request.context.request_id,
        "idempotency_key": request.context.idempotency_key,
        "calculation": {
            "type": "natal",
            "zodiacal_reference_system": "tropical",
            "coordinate_reference_system": "geocentric",
            "house_system": "placidus"
        },
        "birth": {
            "date": request.birth.date,
            "time": time,
            "timezone": timezone,
            "location": {
                "label": location.label,
                "latitude": location.latitude,
                "longitude": location.longitude
            }
        },
        "projection": {
            "level": policy.projection_level()
        }
    }))
}

fn build_llm_request(
    request: &NatalReadingRequestV2,
    calculation: &Value,
    policy: &NatalGatewayPolicy,
) -> Result<GenerateReadingRequest, GatewayError> {
    let audience = resolve_audience_level(request, policy)?;
    match policy.variant {
        NatalVariant::Simplified => build_reading_request(
            calculation,
            &request.context.target_language_code,
            audience,
        )
        .map_err(|err| GatewayError::bad_request(err.detail().message.clone())),
        NatalVariant::Full => {
            validate_engine_response(calculation)
                .map_err(|err| GatewayError::bad_request(err.detail().message.clone()))?;
            build_reading_request_from_engine(
                calculation,
                policy.interpretation_profile_code(),
                &request.context.target_language_code,
                audience,
                None,
                None,
            )
            .map_err(|err| GatewayError::bad_request(err.detail().message.clone()))
        }
    }
}

fn resolve_audience_level(
    request: &NatalReadingRequestV2,
    policy: &NatalGatewayPolicy,
) -> Result<AudienceLevel, GatewayError> {
    match request.context.audience_level.trim().to_ascii_lowercase().as_str() {
        "" | "general" => Ok(policy.default_audience_level()),
        "beginner" => Ok(AudienceLevel::Beginner),
        "intermediate" => Ok(AudienceLevel::Intermediate),
        "expert" => Ok(AudienceLevel::Expert),
        other => Err(GatewayError::bad_request(format!(
            "unsupported audience_level: {other}"
        ))),
    }
}

fn calculation_contract_version(calculation: &Value) -> Option<String> {
    calculation
        .pointer("/audit_payload/contract_version")
        .or_else(|| calculation.pointer("/payload_contract/version"))
        .or_else(|| calculation.pointer("/response_contract_version"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn reading_completeness_hint(calculation: &Value) -> Option<String> {
    calculation
        .pointer("/reading_hint/reading_completeness")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            calculation
                .pointer("/calculation_result/status")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}
