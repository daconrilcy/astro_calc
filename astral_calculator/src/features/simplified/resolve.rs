use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

use super::catalog::SimplifiedCatalog;
use super::request::AstroSimplifiedNatalRequest;
use crate::runtime::RuntimeError;

#[derive(Debug, Clone)]
pub struct ResolvedSimplifiedInput {
    pub input_precision_level: String,
    pub computed_scope: String,
    pub limitations: Vec<String>,
    pub excluded_features: Vec<String>,
    pub birth_date: NaiveDate,
    pub birth_time: Option<NaiveTime>,
    pub timezone: Option<Tz>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub house_system_code: String,
    pub zodiac_key: String,
}

const STABLE_SCOPE: &str = "stable_birth_date_profile";
const PLANETARY_SCOPE: &str = "planetary_positions";
const ANGULAR_SCOPE: &str = "angular_chart";

const EXCLUDED_WITHOUT_ANGULAR: &[&str] = &["ascendant", "houses", "sect", "house_placements"];

pub fn validate_and_resolve(
    request: &AstroSimplifiedNatalRequest,
    catalog: &SimplifiedCatalog,
) -> Result<ResolvedSimplifiedInput, RuntimeError> {
    if request.request_contract_version != super::request::SIMPLIFIED_REQUEST_CONTRACT_VERSION {
        return Err(RuntimeError::InvalidEngineRequest(format!(
            "unsupported request_contract_version: {}",
            request.request_contract_version
        )));
    }

    let birth_date = NaiveDate::parse_from_str(&request.birth.date, "%Y-%m-%d")
        .map_err(|_| RuntimeError::InvalidEngineRequest("birth.date must be YYYY-MM-DD".into()))?;

    if let Some(location) = &request.birth.location {
        validate_coordinates(location.latitude, location.longitude)?;
    }

    let birth_time = match request.birth.time.as_deref() {
        Some(raw) => Some(parse_time(raw)?),
        None => None,
    };

    let timezone = match request.birth.timezone.as_deref() {
        Some(raw) => Some(parse_timezone(raw)?),
        None => {
            if birth_time.is_some() {
                return Err(RuntimeError::InvalidEngineRequest(
                    "birth.time requires birth.timezone".into(),
                ));
            }
            None
        }
    };

    let location_provided = request.birth.location.is_some();
    let time_provided = birth_time.is_some();
    let timezone_provided = timezone.is_some();

    let (latitude, longitude) = request
        .birth
        .location
        .as_ref()
        .map(|loc| (Some(loc.latitude), Some(loc.longitude)))
        .unwrap_or((None, None));

    let input_precision_level =
        classify_input_precision(location_provided, time_provided, timezone_provided);

    let computed_scope = match input_precision_level.as_str() {
        "datetime_without_location" => PLANETARY_SCOPE,
        "complete_birth_data" => ANGULAR_SCOPE,
        _ => STABLE_SCOPE,
    };

    let mut limitations = Vec::new();
    if !time_provided {
        limitations.push("birth_time_missing".to_string());
    }
    if location_provided && !timezone_provided && !time_provided {
        limitations.push("location_provided_without_usable_timezone".to_string());
    }
    if time_provided && timezone_provided && !location_provided {
        limitations.push("location_missing_for_ascendant_and_houses".to_string());
    }

    let excluded_features = if computed_scope == ANGULAR_SCOPE {
        Vec::new()
    } else {
        EXCLUDED_WITHOUT_ANGULAR
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    if catalog.input_precision(&input_precision_level).is_none() {
        return Err(RuntimeError::Ephemeris(format!(
            "unknown input_precision level in catalog: {input_precision_level}"
        )));
    }
    if catalog.scope(computed_scope).is_none() {
        return Err(RuntimeError::Ephemeris(format!(
            "unknown computed_scope in catalog: {computed_scope}"
        )));
    }

    Ok(ResolvedSimplifiedInput {
        input_precision_level,
        computed_scope: computed_scope.to_string(),
        limitations,
        excluded_features,
        birth_date,
        birth_time,
        timezone,
        latitude,
        longitude,
        house_system_code: request.calculation.house_system.clone(),
        zodiac_key: request.calculation.zodiacal_reference_system.clone(),
    })
}

fn classify_input_precision(
    location_provided: bool,
    time_provided: bool,
    timezone_provided: bool,
) -> String {
    match (time_provided, timezone_provided, location_provided) {
        (true, true, true) => "complete_birth_data".to_string(),
        (true, true, false) => "datetime_without_location".to_string(),
        (false, true, true) => "date_with_location_and_timezone_without_time".to_string(),
        (false, true, false) => "date_with_timezone_without_time".to_string(),
        (false, false, true) => "date_with_location_without_timezone".to_string(),
        (false, false, false) => "date_only".to_string(),
        (true, false, _) => "datetime_without_location".to_string(),
    }
}

fn parse_time(raw: &str) -> Result<NaiveTime, RuntimeError> {
    NaiveTime::parse_from_str(raw, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(raw, "%H:%M"))
        .map_err(|_| RuntimeError::InvalidEngineRequest("birth.time must be HH:MM:SS".into()))
}

fn parse_timezone(raw: &str) -> Result<Tz, RuntimeError> {
    raw.parse()
        .map_err(|_| RuntimeError::InvalidEngineRequest(format!("invalid timezone: {raw}")))
}

fn validate_coordinates(latitude: f64, longitude: f64) -> Result<(), RuntimeError> {
    if !(-90.0..=90.0).contains(&latitude) {
        return Err(RuntimeError::InvalidEngineRequest(
            "birth.location.latitude must be between -90 and 90".into(),
        ));
    }
    if !(-180.0..=180.0).contains(&longitude) {
        return Err(RuntimeError::InvalidEngineRequest(
            "birth.location.longitude must be between -180 and 180".into(),
        ));
    }
    Ok(())
}

pub fn declared_datetime_utc(
    resolved: &ResolvedSimplifiedInput,
) -> Result<Option<DateTime<Utc>>, RuntimeError> {
    let Some(time) = resolved.birth_time else {
        return Ok(None);
    };
    let Some(tz) = resolved.timezone else {
        return Err(RuntimeError::InvalidEngineRequest(
            "birth.time requires birth.timezone".into(),
        ));
    };
    let naive = resolved.birth_date.and_time(time);
    tz.from_local_datetime(&naive)
        .single()
        .ok_or_else(|| {
            RuntimeError::InvalidEngineRequest(
                "ambiguous or invalid local birth datetime for timezone".into(),
            )
        })
        .map(|dt| dt.with_timezone(&Utc))
        .map(Some)
}

pub fn build_uncertainty_window(
    resolved: &ResolvedSimplifiedInput,
    catalog: &SimplifiedCatalog,
) -> Result<(DateTime<Utc>, DateTime<Utc>), RuntimeError> {
    if let Some(instant) = declared_datetime_utc(resolved)? {
        return Ok((instant, instant));
    }

    if let Some(tz) = resolved.timezone {
        let start_local = tz
            .from_local_datetime(
                &resolved
                    .birth_date
                    .and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
            )
            .single()
            .ok_or_else(|| {
                RuntimeError::InvalidEngineRequest("invalid local day start for timezone".into())
            })?;
        let end_local = tz
            .from_local_datetime(
                &resolved
                    .birth_date
                    .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap()),
            )
            .single()
            .ok_or_else(|| {
                RuntimeError::InvalidEngineRequest("invalid local day end for timezone".into())
            })?;
        return Ok((
            start_local.with_timezone(&Utc),
            end_local.with_timezone(&Utc),
        ));
    }

    if catalog.policy.date_only_uncertainty_mode != "world_civil_date_window" {
        return Err(RuntimeError::Ephemeris(format!(
            "unsupported date_only_uncertainty_mode: {}",
            catalog.policy.date_only_uncertainty_mode
        )));
    }

    let previous_day = resolved.birth_date - Duration::days(1);
    let next_day = resolved.birth_date + Duration::days(1);
    let start =
        Utc.from_utc_datetime(&previous_day.and_time(NaiveTime::from_hms_opt(10, 0, 0).unwrap()));
    let end = Utc.from_utc_datetime(&next_day.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
    Ok((start, end))
}
