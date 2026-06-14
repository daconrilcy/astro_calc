use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::domain::NatalChartInput;
use crate::engine::request::{AstroEngineRequest, REQUEST_CONTRACT_VERSION};
use crate::runtime::RuntimeError;

pub struct ResolvedEngineRequest {
    pub natal_input: NatalChartInput,
    pub projection_level: String,
    pub birth_datetime_local: String,
    pub birth_timezone: String,
    pub birth_datetime_utc: DateTime<Utc>,
    pub location_label: String,
    pub zodiac_key: String,
    pub coordinate_key: String,
    pub house_system_code: String,
    pub calculation_type: String,
}

pub fn validate_request_early(request: &AstroEngineRequest) -> Result<(), RuntimeError> {
    if request.request_contract_version != REQUEST_CONTRACT_VERSION {
        return Err(RuntimeError::InvalidEngineRequest(format!(
            "unsupported request_contract_version: {}",
            request.request_contract_version
        )));
    }

    if request.calculation.calculation_type != "natal" {
        return Err(RuntimeError::InvalidEngineRequest(format!(
            "unsupported calculation type: {}",
            request.calculation.calculation_type
        )));
    }

    if !matches!(
        request.projection.level.as_str(),
        "compact" | "standard" | "rich" | "expert"
    ) {
        return Err(RuntimeError::InvalidEngineRequest(format!(
            "unsupported projection level: {}",
            request.projection.level
        )));
    }

    if let Some(contract) = request.projection.contract_version.as_deref() {
        if contract != "llm_projection_natal_v1" {
            return Err(RuntimeError::InvalidEngineRequest(format!(
                "unsupported projection contract_version: {contract}"
            )));
        }
    }

    Ok(())
}

pub fn local_birth_to_utc(
    date: &str,
    time: &str,
    timezone: &str,
) -> Result<DateTime<Utc>, RuntimeError> {
    let naive_date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| {
        RuntimeError::InvalidEngineRequest("birth.date must be YYYY-MM-DD".to_string())
    })?;
    let naive_time = NaiveTime::parse_from_str(time, "%H:%M:%S").map_err(|_| {
        RuntimeError::InvalidEngineRequest("birth.time must be HH:MM:SS".to_string())
    })?;
    let naive_local = NaiveDateTime::new(naive_date, naive_time);

    let tz: Tz = timezone
        .parse()
        .map_err(|_| RuntimeError::InvalidEngineRequest(format!("invalid timezone: {timezone}")))?;

    tz.from_local_datetime(&naive_local)
        .single()
        .ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(
                "ambiguous or invalid local birth datetime for timezone".to_string(),
            )
        })
        .map(|local| local.with_timezone(&Utc))
}

pub fn validate_and_resolve_request(
    request: &AstroEngineRequest,
    reference_version_id: i32,
    zodiacal_reference_system_id: i32,
    coordinate_reference_system_id: i32,
    house_system_id: i32,
) -> Result<ResolvedEngineRequest, RuntimeError> {
    validate_request_early(request)?;
    let projection_level = request.projection.level.clone();

    let birth_datetime_utc = local_birth_to_utc(
        &request.birth.date,
        &request.birth.time,
        &request.birth.timezone,
    )?;
    let naive_local = NaiveDateTime::new(
        NaiveDate::parse_from_str(&request.birth.date, "%Y-%m-%d").map_err(|_| {
            RuntimeError::InvalidEngineRequest("birth.date must be YYYY-MM-DD".to_string())
        })?,
        NaiveTime::parse_from_str(&request.birth.time, "%H:%M:%S").map_err(|_| {
            RuntimeError::InvalidEngineRequest("birth.time must be HH:MM:SS".to_string())
        })?,
    );

    let location_label = request
        .birth
        .location
        .label
        .clone()
        .filter(|label| !label.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "{:.4}, {:.4}",
                request.birth.location.latitude, request.birth.location.longitude
            )
        });

    Ok(ResolvedEngineRequest {
        natal_input: NatalChartInput {
            subject_label: request.request_id.clone(),
            birth_datetime_utc,
            latitude_deg: request.birth.location.latitude,
            longitude_deg: request.birth.location.longitude,
            altitude_m: None,
            reference_version_id,
            calculation_profile_id: None,
            zodiacal_reference_system_id,
            coordinate_reference_system_id,
            house_system_id,
            product_code: Some("basic".to_string()),
            client_idempotency_key: request
                .idempotency_key
                .clone()
                .filter(|key| !key.trim().is_empty()),
        },
        projection_level,
        birth_datetime_local: naive_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        birth_timezone: request.birth.timezone.clone(),
        birth_datetime_utc,
        location_label,
        zodiac_key: request.calculation.zodiacal_reference_system.clone(),
        coordinate_key: request.calculation.coordinate_reference_system.clone(),
        house_system_code: request.calculation.house_system.clone(),
        calculation_type: request.calculation.calculation_type.clone(),
    })
}
