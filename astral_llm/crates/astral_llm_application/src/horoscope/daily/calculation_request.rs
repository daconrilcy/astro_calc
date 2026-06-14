use super::*;

pub fn build_calculation_request(
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    build_calculation_request_for_service(HOROSCOPE_SERVICE_CODE, request)
}

pub fn build_calculation_request_for_service(
    service_code: &str,
    request: &HoroscopePublicRequest,
) -> Result<serde_json::Value, GenerationError> {
    let payload = serde_json::to_value(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PAYLOAD_INVALID: {err}"),
            Value::Null,
        )
    })?;
    let calculation_request =
        astral_calculator::horoscope::build_horoscope_daily_calculation_request_from_public(
            service_code,
            &payload,
        )
        .map_err(|message| {
            GenerationError::with_details(GenerationErrorCode::InvalidInput, message, Value::Null)
        })?;
    serde_json::to_value(calculation_request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_CALCULATION_FAILED: {err}"),
            Value::Null,
        )
    })
}
