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
    validate_supported_service_code(service_code)?;
    validate_public_request_for_service(service_code, request)?;
    let refs = ReferenceData::load(service_code)?;
    let slots = slot_profiles(service_code)?;
    let mut out = json!({        "contract_version": "horoscope_calculation_request_v1",        "service_code": service_code,        "chart_calculation_id": request.chart_calculation_id,        "period": {            "date": request.date,            "timezone": request.timezone        },        "slots": slots.into_iter().map(|slot| json!({            "slot_code": slot.slot_code,            "start_local_time": slot.start_local_time,            "end_local_time": slot.end_local_time,            "reference_local_time": slot.reference_local_time        })).collect::<Vec<_>>()    });
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        out["location"] = serde_json::to_value(
            request
                .location
                .as_ref()
                .ok_or_else(|| horoscope_error("HOROSCOPE_LOCATION_REQUIRED"))?,
        )
        .expect("location serializes");
        out["slot_profile_code"] = json!("daily_2h_slots");
        out["house_system_code"] = json!(refs
            .service_profile
            .house_system_code
            .as_deref()
            .ok_or_else(|| horoscope_error("HOROSCOPE_SCORING_FAILED"))?);
        out["calculation_features"] = json!([
            "sky_snapshot",
            "moon_context",
            "natal_transits",
            "natal_house_activations",
            "local_chart",
            "local_angles",
            "local_houses",
            "local_house_placements"
        ]);
    }
    Ok(out)
}
pub(crate) fn validate_public_request_for_service(
    service_code: &str,
    request: &HoroscopePublicRequest,
) -> Result<(), GenerationError> {
    if service_code != HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        return Ok(());
    }
    let location = request
        .location
        .as_ref()
        .ok_or_else(|| horoscope_error("HOROSCOPE_LOCATION_REQUIRED"))?;
    if !(-90.0..=90.0).contains(&location.latitude)
        || !(-180.0..=180.0).contains(&location.longitude)
    {
        return Err(GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            "HOROSCOPE_PAYLOAD_INVALID: location latitude/longitude out of range",
            Value::Null,
        ));
    }
    Ok(())
}
