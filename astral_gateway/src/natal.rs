use std::sync::Arc;

use astral_contracts::{
    NatalProductCode, NatalVariant, ProductTier, QualityMetadataCommon, ResponseMetadataCommon,
};
use serde_json::{json, Value};

use crate::contracts::NatalInspectionResponseV2;
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

    pub fn default_audience_level(&self) -> &'static str {
        match self.tier {
            ProductTier::Free => "beginner",
            ProductTier::Basic => "intermediate",
            ProductTier::Premium => "expert",
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
            NatalVariant::Simplified => "natal_simplified",
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
                validate_simplified_calculation_request(&calculation_request)?;
                self.calculator
                    .calculate_simplified_natal(&calculation_request)
                    .await?
            }
            NatalVariant::Full => {
                let calculation_request = full_calculation_request(&request, &policy)?;
                self.calculator
                    .calculate_full_natal(&calculation_request)
                    .await?
            }
        };

        let reading_request = build_llm_request(&request, &calculation, &policy)?;
        let reading = self.llm.generate_reading(&reading_request).await?;
        let debug = build_natal_debug_payload(&reading_request, &reading)?;

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
            debug: Some(debug),
        })
    }

    pub async fn inspect(
        &self,
        policy: NatalGatewayPolicy,
        request: NatalReadingRequestV2,
    ) -> Result<NatalInspectionResponseV2, GatewayError> {
        let calculation = match policy.variant {
            NatalVariant::Simplified => {
                let calculation_request = simplified_calculation_request(&request)?;
                validate_simplified_calculation_request(&calculation_request)?;
                self.calculator
                    .calculate_simplified_natal(&calculation_request)
                    .await?
            }
            NatalVariant::Full => {
                let calculation_request = full_calculation_request(&request, &policy)?;
                self.calculator
                    .calculate_full_natal(&calculation_request)
                    .await?
            }
        };

        let reading_request = build_llm_request(&request, &calculation, &policy)?;
        Ok(NatalInspectionResponseV2 {
            metadata: ResponseMetadataCommon {
                product_code: policy.product_code().as_str().to_string(),
                tier: policy.tier,
                variant: policy.variant.as_str().to_string(),
                contract_version: "natal_inspection_response_v2".to_string(),
            },
            quality: QualityMetadataCommon {
                calculator_contract_version: calculation_contract_version(&calculation),
                llm_contract_version: Some("generate_reading_request_v1".to_string()),
                reading_completeness: reading_completeness_hint(&calculation),
            },
            calculation,
            llm_request: serde_json::to_value(reading_request).map_err(|err| {
                GatewayError::Internal(format!("llm request serialization failed: {err}"))
            })?,
        })
    }
}

fn simplified_calculation_request(request: &NatalReadingRequestV2) -> Result<Value, GatewayError> {
    let mut birth = serde_json::Map::new();
    birth.insert(
        "date".to_string(),
        Value::String(request.birth.date.clone()),
    );
    if let Some(time) = &request.birth.time {
        birth.insert("time".to_string(), Value::String(time.clone()));
    }
    if let Some(timezone) = &request.birth.timezone {
        birth.insert("timezone".to_string(), Value::String(timezone.clone()));
    }
    if let Some(location) = &request.birth.location {
        birth.insert(
            "location".to_string(),
            serde_json::to_value(location).map_err(|err| {
                GatewayError::bad_request(format!("invalid birth.location payload: {err}"))
            })?,
        );
    }

    Ok(serde_json::json!({
        "request_contract_version": "astro_simplified_natal_request_v1",
        "request_id": request.context.request_id,
        "birth": Value::Object(birth),
        "calculation": {
            "zodiacal_reference_system": "tropical",
            "house_system": "placidus"
        }
    }))
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
    let timezone =
        request.birth.timezone.clone().ok_or_else(|| {
            GatewayError::bad_request("birth.timezone is required for full natal")
        })?;
    let location =
        request.birth.location.clone().ok_or_else(|| {
            GatewayError::bad_request("birth.location is required for full natal")
        })?;

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

fn validate_simplified_calculation_request(value: &Value) -> Result<(), GatewayError> {
    let version = value
        .get("request_contract_version")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::bad_request("request_contract_version is required"))?;
    if version != "astro_simplified_natal_request_v1" {
        return Err(GatewayError::bad_request(format!(
            "unsupported request_contract_version: {version}"
        )));
    }
    let date = value
        .pointer("/birth/date")
        .and_then(Value::as_str)
        .ok_or_else(|| GatewayError::bad_request("birth.date is required"))?;
    if !date.chars().all(|c| c.is_ascii_digit() || c == '-') || date.len() != 10 {
        return Err(GatewayError::bad_request("birth.date must be YYYY-MM-DD"));
    }
    if let Some(location) = value.get("birth").and_then(|birth| birth.get("location")) {
        if location.get("latitude").and_then(Value::as_f64).is_none()
            || location.get("longitude").and_then(Value::as_f64).is_none()
        {
            return Err(GatewayError::bad_request(
                "birth.location requires latitude and longitude",
            ));
        }
    }
    if value
        .pointer("/birth/time")
        .and_then(Value::as_str)
        .is_some()
        && value
            .pointer("/birth/timezone")
            .and_then(Value::as_str)
            .is_none()
    {
        return Err(GatewayError::bad_request(
            "birth.time requires birth.timezone",
        ));
    }
    Ok(())
}

fn validate_engine_response(engine: &Value) -> Result<(), GatewayError> {
    for key in [
        "response_contract_version",
        "calculation_result",
        "audit_payload",
    ] {
        if engine.get(key).is_none() {
            return Err(GatewayError::bad_request(format!(
                "engine response missing {key}"
            )));
        }
    }
    let version = engine
        .get("response_contract_version")
        .and_then(Value::as_str)
        .unwrap_or("");
    if version != "astro_engine_response_v1" {
        return Err(GatewayError::bad_request(format!(
            "unsupported response_contract_version: {version}"
        )));
    }
    let status = engine
        .pointer("/calculation_result/status")
        .and_then(Value::as_str)
        .unwrap_or("");
    if status != "completed" {
        return Err(GatewayError::bad_request(format!(
            "engine calculation not completed: {status}"
        )));
    }
    if engine
        .pointer("/audit_payload/contract_version")
        .and_then(Value::as_str)
        .is_none()
    {
        return Err(GatewayError::bad_request(
            "audit_payload.contract_version is required",
        ));
    }
    if engine.pointer("/audit_payload/payload").is_none() {
        return Err(GatewayError::bad_request(
            "audit_payload.payload is required",
        ));
    }
    Ok(())
}

fn build_simplified_llm_request(
    calculation: &Value,
    user_language: &str,
    audience_level: &str,
) -> Result<Value, GatewayError> {
    let payload = calculation
        .pointer("/simplified_payload/payload")
        .ok_or_else(|| {
            GatewayError::bad_request("calculator response missing simplified_payload.payload")
        })?
        .clone();

    let mut data = payload;
    let mut forbidden_wording = Vec::new();
    let mut custom_instructions = None;
    let mut chapter_code = "identity";
    if let Some(controls) = calculation.get("llm_payload") {
        if let Some(obj) = data.as_object_mut() {
            obj.insert("llm_controls".into(), controls.clone());
            scrub_simplified_payload_for_llm(obj, controls);
        }
        forbidden_wording = blocked_interpretation_fact_codes(controls);
        if sun_sign_blocked(controls) {
            chapter_code = "ambiguous_core_identity";
            custom_instructions = Some(
                "Le Soleil est ambigu (sun.sign bloqué). N'affirmez aucun signe solaire. \
                 Expliquez la zone de changement possible entre signes, puis seulement les \
                 placements stables secondaires (Mercure, Vénus, Mars…) avec prudence."
                    .to_string(),
            );
        }
    }

    let chapter_title = if chapter_code == "ambiguous_core_identity" {
        "Identité — Soleil ambigu"
    } else {
        "Identité"
    };

    Ok(json!({
        "request_id": calculation.get("request_id").and_then(Value::as_str),
        "idempotency_key": null,
        "product_context": {
            "product_code": "natal_prompter",
            "interpretation_profile_code": "natal_simplified",
            "user_language": user_language,
            "audience_level": audience_level
        },
        "astro_result": {
            "contract_version": "natal_simplified_structured_v1",
            "chart_type": "natal",
            "data": data
        },
        "astrologer_profile": {
            "profile_id": null,
            "name": null,
            "tone": "warm",
            "jargon_level": "beginner",
            "wording_style": "clear",
            "preferred_domains": [],
            "forbidden_wording": forbidden_wording,
            "custom_instructions": custom_instructions
        },
        "engine": {
            "domain_count": 1,
            "allow_fallback": true
        },
        "response_contract": {
            "output_schema_version": "natal_reading_v1",
            "generation_mode": "single_pass",
            "format": "structured_json",
            "chapters": [{
                "code": chapter_code,
                "title": chapter_title,
                "min_words": null,
                "max_words": null,
                "target_tokens": null,
                "required_fields": []
            }],
            "global_max_tokens": null,
            "include_astro_sources": false,
            "include_legal_disclaimer": true
        },
        "safety_policy": null
    }))
}

fn build_engine_llm_request(
    engine: &Value,
    profile_code: &str,
    user_language: &str,
    audience_level: &str,
) -> Result<Value, GatewayError> {
    validate_engine_response(engine)?;
    let contract_version = engine
        .pointer("/audit_payload/contract_version")
        .and_then(Value::as_str)
        .unwrap_or("");
    let payload = engine
        .pointer("/audit_payload/payload")
        .cloned()
        .unwrap_or(Value::Null);
    Ok(json!({
        "request_id": engine.get("request_id").and_then(Value::as_str),
        "idempotency_key": null,
        "product_context": {
            "product_code": "natal_prompter",
            "interpretation_profile_code": profile_code,
            "user_language": user_language,
            "audience_level": audience_level
        },
        "astro_result": {
            "contract_version": contract_version,
            "chart_type": "natal",
            "data": payload
        },
        "astrologer_profile": {
            "profile_id": null,
            "name": null,
            "tone": "warm",
            "jargon_level": "beginner",
            "wording_style": "clear",
            "preferred_domains": [
                "identity",
                "emotional_life",
                "relationships",
                "career",
                "growth_path"
            ],
            "forbidden_wording": [],
            "custom_instructions": null
        },
        "engine": {
            "allow_fallback": true
        },
        "response_contract": {
            "output_schema_version": "natal_reading_v1",
            "generation_mode": "chapter_orchestrated",
            "format": "structured_json",
            "chapters": [],
            "global_max_tokens": null,
            "include_astro_sources": true,
            "include_legal_disclaimer": true
        },
        "safety_policy": null
    }))
}

fn blocked_interpretation_fact_codes(controls: &Value) -> Vec<String> {
    controls
        .get("blocked_interpretation_fact_codes")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn sun_sign_blocked(controls: &Value) -> bool {
    controls
        .get("blocked_interpretation_fact_codes")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|v| v.as_str() == Some("sun.sign")))
}

fn blocked_object_codes(controls: &Value) -> Vec<String> {
    blocked_interpretation_fact_codes(controls)
        .into_iter()
        .filter_map(|code| code.strip_suffix(".sign").map(str::to_string))
        .collect()
}

fn scrub_simplified_payload_for_llm(
    payload: &mut serde_json::Map<String, Value>,
    controls: &Value,
) {
    payload.remove("position_count");
    payload.remove("house_cusp_count");
    payload.remove("aspect_count");

    let blocked = blocked_object_codes(controls);
    if blocked.is_empty() {
        return;
    }
    if let Some(objects) = payload.get_mut("objects").and_then(Value::as_array_mut) {
        for object in objects {
            let object_code = object
                .get("object_code")
                .and_then(Value::as_str)
                .unwrap_or("");
            if blocked.iter().any(|code| code == object_code) {
                if let Some(map) = object.as_object_mut() {
                    map.remove("sign_code");
                    map.remove("sign_name");
                    map.remove("longitude_deg");
                }
            }
        }
    }
}

fn build_llm_request(
    request: &NatalReadingRequestV2,
    calculation: &Value,
    policy: &NatalGatewayPolicy,
) -> Result<Value, GatewayError> {
    let audience = resolve_audience_level(request, policy)?;
    match policy.variant {
        NatalVariant::Simplified => build_simplified_llm_request(
            calculation,
            &request.context.target_language_code,
            audience,
        ),
        NatalVariant::Full => {
            validate_engine_response(calculation)?;
            build_engine_llm_request(
                calculation,
                policy.interpretation_profile_code(),
                &request.context.target_language_code,
                audience,
            )
        }
    }
}

fn resolve_audience_level(
    request: &NatalReadingRequestV2,
    policy: &NatalGatewayPolicy,
) -> Result<&'static str, GatewayError> {
    match request
        .context
        .audience_level
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "" | "general" => Ok(policy.default_audience_level()),
        "beginner" => Ok("beginner"),
        "intermediate" => Ok("intermediate"),
        "expert" => Ok("expert"),
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

fn build_natal_debug_payload(
    reading_request: &Value,
    reading: &Value,
) -> Result<Value, GatewayError> {
    Ok(serde_json::json!({
        "run_id": reading.get("run_id").and_then(Value::as_str),
        "llm_request": reading_request,
    }))
}
