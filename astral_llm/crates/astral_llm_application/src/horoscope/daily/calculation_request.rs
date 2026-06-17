use super::*;
use astral_calculator::features::horoscope::{
    HoroscopeCalculationRequest, HoroscopeCalculationSlotRequest,
    HoroscopeLocation as CalcHoroscopeLocation, HoroscopePeriod,
};

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
    let request = validate_public_request(&serde_json::to_value(request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_PAYLOAD_INVALID: {err}"),
            Value::Null,
        )
    })?)?;
    let service_profile = service_profile(service_code)?;
    let mut slots = slot_profiles(service_code)?;

    let location = if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        let location = request
            .location
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
        Some(CalcHoroscopeLocation {
            latitude: location.latitude,
            longitude: location.longitude,
            label: location.label,
        })
    } else {
        None
    };

    let slot_profile_code = if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        Some(
            service_profile
                .time_slot_profile_code
                .clone()
                .ok_or_else(|| horoscope_error("HOROSCOPE_SERVICE_NOT_IMPLEMENTED"))?,
        )
    } else {
        None
    };

    let calculation_features =
        if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
            vec![
                "sky_snapshot".into(),
                "moon_context".into(),
                "natal_transits".into(),
                "natal_house_activations".into(),
                "local_chart".into(),
                "local_angles".into(),
                "local_houses".into(),
                "local_house_placements".into(),
            ]
        } else {
            Vec::new()
        };

    slots.sort_by_key(|slot| slot.sort_order);
    let slots = slots
        .into_iter()
        .map(|slot| HoroscopeCalculationSlotRequest {
            slot_code: slot.slot_code,
            start_local_time: slot.start_local_time,
            end_local_time: slot.end_local_time,
            reference_local_time: slot.reference_local_time,
        })
        .collect::<Vec<_>>();

    let calculation_request = HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".into(),
        service_code: service_code.to_string(),
        period: HoroscopePeriod {
            date: request.date.clone(),
            timezone: request.timezone.clone(),
        },
        chart_calculation_id: request.chart_calculation_id,
        location,
        slot_profile_code,
        house_system_code: if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
            service_profile.house_system_code
        } else {
            None
        },
        calculation_features,
        slots,
    };

    serde_json::to_value(calculation_request).map_err(|err| {
        GenerationError::with_details(
            GenerationErrorCode::InvalidInput,
            format!("HOROSCOPE_CALCULATION_FAILED: {err}"),
            Value::Null,
        )
    })
}
