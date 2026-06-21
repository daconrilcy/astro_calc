//! Module astral_calculator\src\features\horoscope\builders.rs du moteur astral_calculator.

use std::collections::HashSet;

use crate::application::ports::{HoroscopeBuilderCatalog, HoroscopePeriodProfile};
use crate::shared::time::{local_to_utc, require_canonical_utc_offset};
use chrono::{NaiveDate, NaiveTime};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    HoroscopeCalculationRequest, HoroscopeCalculationSlotRequest, HoroscopeLocation,
    HoroscopePeriod, HoroscopePeriodCalculationRequest, HoroscopeScanPlan,
    HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE, HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_FREE_DAILY_SERVICE_CODE, HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE,
    HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE,
    HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE,
};

#[derive(Debug, Clone, Deserialize)]
/// Structure DailyPublicRequest.
struct DailyPublicRequest {
    date: String,
    timezone: String,
    chart_calculation_id: String,
    #[serde(default)]
    location: Option<HoroscopeLocation>,
}

#[derive(Debug, Clone, Deserialize)]
/// Structure PeriodPublicRequest.
struct PeriodPublicRequest {
    anchor_date: String,
    timezone: String,
    chart_calculation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
/// Structure SlotProfileRow.
struct SlotProfileRow {
    slot_code: String,
    start_local_time: String,
    end_local_time: String,
    reference_local_time: String,
    sort_order: i32,
}

#[derive(Debug, Clone)]
/// Structure ServiceProfile.
struct ServiceProfile {
    house_system_code: Option<String>,
    period_profile_code: Option<String>,
    scan_profile_code: Option<String>,
}

#[derive(Debug, Clone)]
/// Structure ScanProfile.
struct ScanProfile {
    granularity: String,
    reference_time_local: String,
    expected_snapshots_per_day: usize,
}

/// Fonction build_horoscope_daily_calculation_request_from_public.
pub async fn build_horoscope_daily_calculation_request_from_public<R>(
    repository: &R,
    service_code: &str,
    payload: &Value,
) -> Result<HoroscopeCalculationRequest, String>
where
    R: HoroscopeBuilderCatalog,
{
    validate_daily_service_code(service_code)?;
    let request: DailyPublicRequest =
        serde_json::from_value(payload.clone()).map_err(|err| err.to_string())?;
    validate_daily_public_request(service_code, &request)?;
    let profile = service_profile(repository, service_code).await?;
    let mut slots = slot_profiles(repository, service_code).await?;
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(HoroscopeCalculationRequest {
        contract_version: "horoscope_calculation_request".into(),
        service_code: service_code.to_string(),
        chart_calculation_id: request.chart_calculation_id,
        period: HoroscopePeriod {
            date: request.date,
            timezone: request.timezone,
        },
        location: if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
            request.location
        } else {
            None
        },
        slot_profile_code: (service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE)
            .then_some("daily_2h_slots".to_string()),
        house_system_code: if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
            profile.house_system_code
        } else {
            None
        },
        calculation_features: if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
        {
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
        },
        slots: slots
            .into_iter()
            .map(|slot| HoroscopeCalculationSlotRequest {
                slot_code: slot.slot_code,
                start_local_time: slot.start_local_time,
                end_local_time: slot.end_local_time,
                reference_local_time: slot.reference_local_time,
            })
            .collect(),
    })
}

/// Fonction build_horoscope_period_calculation_request_from_public.
pub async fn build_horoscope_period_calculation_request_from_public<R>(
    repository: &R,
    service_code: &str,
    payload: &Value,
) -> Result<HoroscopePeriodCalculationRequest, String>
where
    R: HoroscopeBuilderCatalog,
{
    validate_period_service_code(service_code)?;
    let request: PeriodPublicRequest =
        serde_json::from_value(payload.clone()).map_err(|err| err.to_string())?;
    NaiveDate::parse_from_str(&request.anchor_date, "%Y-%m-%d")
        .map_err(|_| "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED".to_string())?;
    request
        .timezone
        .parse::<Tz>()
        .map_err(|_| "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED".to_string())?;

    let profile = period_service_profile(repository, service_code).await?;
    let period_profile_code = profile
        .period_profile_code
        .ok_or_else(|| "HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED".to_string())?;
    let scan_profile_code = profile
        .scan_profile_code
        .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
    let period_resolution = resolve_period_window(
        repository,
        &period_profile_code,
        &request.anchor_date,
        &request.timezone,
    )
    .await?;
    let scan_plan = build_scan_plan(repository, &period_resolution, &scan_profile_code).await?;
    validate_scan_plan_value(repository, &period_resolution, &scan_plan).await?;

    let scan_plan: HoroscopeScanPlan =
        serde_json::from_value(scan_plan).map_err(|err| err.to_string())?;
    Ok(HoroscopePeriodCalculationRequest {
        contract_version: "horoscope_period_calculation_request".into(),
        service_code: service_code.to_string(),
        chart_calculation_id: request.chart_calculation_id,
        period_resolution,
        scan_plan,
    })
}

/// Fonction validate_daily_service_code.
fn validate_daily_service_code(service_code: &str) -> Result<(), String> {
    if matches!(
        service_code,
        HOROSCOPE_BASIC_DAILY_NATAL_SERVICE_CODE
            | HOROSCOPE_FREE_DAILY_SERVICE_CODE
            | HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE
    ) {
        Ok(())
    } else {
        Err("HOROSCOPE_SERVICE_NOT_IMPLEMENTED".to_string())
    }
}

/// Fonction validate_period_service_code.
fn validate_period_service_code(service_code: &str) -> Result<(), String> {
    if matches!(
        service_code,
        HOROSCOPE_FREE_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_BASIC_NEXT_7_DAYS_NATAL_SERVICE_CODE
            | HOROSCOPE_PREMIUM_NEXT_7_DAYS_NATAL_SERVICE_CODE
    ) {
        Ok(())
    } else {
        Err("HOROSCOPE_SERVICE_NOT_IMPLEMENTED".to_string())
    }
}

/// Fonction validate_daily_public_request.
fn validate_daily_public_request(
    service_code: &str,
    request: &DailyPublicRequest,
) -> Result<(), String> {
    NaiveDate::parse_from_str(&request.date, "%Y-%m-%d")
        .map_err(|_| "HOROSCOPE_PAYLOAD_INVALID: date must be YYYY-MM-DD".to_string())?;
    request
        .timezone
        .parse::<Tz>()
        .map_err(|_| "HOROSCOPE_PAYLOAD_INVALID: timezone must be an IANA timezone".to_string())?;
    if request.chart_calculation_id.trim().is_empty() {
        return Err("HOROSCOPE_NATAL_CHART_REQUIRED".to_string());
    }
    if service_code == HOROSCOPE_PREMIUM_DAILY_LOCAL_2H_SLOTS_SERVICE_CODE {
        let location = request
            .location
            .as_ref()
            .ok_or_else(|| "HOROSCOPE_LOCATION_REQUIRED".to_string())?;
        if !(-90.0..=90.0).contains(&location.latitude)
            || !(-180.0..=180.0).contains(&location.longitude)
        {
            return Err(
                "HOROSCOPE_PAYLOAD_INVALID: location latitude/longitude out of range".to_string(),
            );
        }
    }
    Ok(())
}

/// Fonction service_profile.
async fn service_profile<R>(repository: &R, service_code: &str) -> Result<ServiceProfile, String>
where
    R: HoroscopeBuilderCatalog,
{
    let row = repository
        .horoscope_service_profiles()
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .find(|row| row.service_code == service_code)
        .ok_or_else(|| "HOROSCOPE_SERVICE_NOT_IMPLEMENTED".to_string())?;
    Ok(ServiceProfile {
        house_system_code: row.house_system_code,
        period_profile_code: row.period_profile_code,
        scan_profile_code: row.scan_profile_code,
    })
}

/// Fonction period_service_profile.
async fn period_service_profile<R>(
    repository: &R,
    service_code: &str,
) -> Result<ServiceProfile, String>
where
    R: HoroscopeBuilderCatalog,
{
    let profile = service_profile(repository, service_code).await?;
    if profile.period_profile_code.is_none() || profile.scan_profile_code.is_none() {
        return Err("HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED".to_string());
    }
    Ok(profile)
}

/// Fonction slot_profiles.
async fn slot_profiles<R>(repository: &R, service_code: &str) -> Result<Vec<SlotProfileRow>, String>
where
    R: HoroscopeBuilderCatalog,
{
    let mut slots = repository
        .horoscope_time_slot_profiles()
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .filter(|row| row.service_code == service_code)
        .map(|row| SlotProfileRow {
            slot_code: row.slot_code,
            start_local_time: row.start_local_time,
            end_local_time: row.end_local_time,
            reference_local_time: row.reference_local_time,
            sort_order: row.sort_order,
        })
        .collect::<Vec<SlotProfileRow>>();
    slots.sort_by_key(|slot| slot.sort_order);
    Ok(slots)
}

/// Fonction resolve_period_window.
async fn resolve_period_window<R>(
    repository: &R,
    period_profile_code: &str,
    anchor_date: &str,
    timezone: &str,
) -> Result<Value, String>
where
    R: HoroscopeBuilderCatalog,
{
    let profiles = repository
        .astral_time_period_profiles()
        .await
        .map_err(|err| err.to_string())?;
    let profile_defs = profiles
        .into_iter()
        .map(map_period_profile_definition)
        .collect::<Result<Vec<_>, _>>()?;
    let resolver = astral_time_window::PeriodWindowResolver::new(profile_defs);
    let request = astral_time_window::PeriodWindowRequest {
        period_profile_code: period_profile_code.to_string(),
        anchor_date: anchor_date.to_string(),
        timezone: timezone.to_string(),
        custom_start_date: None,
        custom_end_date: None,
    };
    let resolved = resolver
        .resolve(&request)
        .map_err(map_period_window_error)?;
    let start_utc = resolved
        .start_datetime_utc()
        .map_err(map_period_window_error)?;
    let end_utc = resolved
        .end_datetime_utc()
        .map_err(map_period_window_error)?;
    Ok(json!({
        "period_profile_code": period_profile_code,
        "anchor_date": anchor_date,
        "timezone": timezone,
        "start_datetime_local": resolved.start_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "end_datetime_local": resolved.end_datetime_local.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "start_datetime_utc": start_utc,
        "end_datetime_utc": end_utc,
        "end_exclusive": resolved.end_exclusive,
        "duration_days": resolved.duration_days,
        "included_dates": resolved.included_dates(),
        "included_days": resolved.included_days
    }))
}

/// Fonction map_period_window_error.
fn map_period_window_error(err: astral_time_window::PeriodWindowError) -> String {
    match err {
        astral_time_window::PeriodWindowError::InvalidTimezone(_) => {
            "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED".to_string()
        }
        astral_time_window::PeriodWindowError::InvalidDate {
            field: "anchor_date",
            ..
        } => "HOROSCOPE_PERIOD_ANCHOR_DATE_REQUIRED".to_string(),
        astral_time_window::PeriodWindowError::InvalidDate { .. }
        | astral_time_window::PeriodWindowError::AmbiguousLocalDateTime { .. }
        | astral_time_window::PeriodWindowError::NonexistentLocalDateTime { .. } => {
            "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string()
        }
        _ => "HOROSCOPE_PERIOD_PROFILE_UNSUPPORTED".to_string(),
    }
}

/// Fonction build_scan_plan.
async fn build_scan_plan<R>(
    repository: &R,
    period_resolution: &Value,
    scan_profile_code: &str,
) -> Result<Value, String>
where
    R: HoroscopeBuilderCatalog,
{
    let scan_profile = scan_profile(repository, scan_profile_code).await?;
    let tz = period_resolution["timezone"]
        .as_str()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?
        .parse::<Tz>()
        .map_err(|_| "HOROSCOPE_PERIOD_TIMEZONE_REQUIRED".to_string())?;
    let dates = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    let reference_times = scan_profile.reference_times()?;
    let mut snapshots = Vec::new();
    for value in dates {
        let date = value
            .as_str()
            .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
        let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
        for time in &reference_times {
            let local = parsed.and_time(*time);
            let utc = local_to_utc(tz, local, "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH")?;
            let time_label = time.format("%H:%M").to_string();
            let key_suffix = if scan_profile_code == "daily_noon_7_days" {
                "noon".to_string()
            } else {
                time_label.clone()
            };
            snapshots.push(json!({
                "snapshot_key": format!("{date}:{key_suffix}"),
                "date": date,
                "reference_time_local": time_label,
                "reference_datetime_local": local.format("%Y-%m-%dT%H:%M:%S").to_string(),
                "reference_datetime_utc": utc
            }));
        }
    }
    let duration_days = period_resolution["duration_days"]
        .as_u64()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?
        as usize;
    if snapshots.len() != duration_days * scan_profile.expected_snapshots_per_day {
        return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
    }
    Ok(json!({
        "scan_profile_code": scan_profile_code,
        "granularity": scan_profile.granularity,
        "snapshot_count": snapshots.len(),
        "snapshots": snapshots
    }))
}

/// Fonction validate_scan_plan_value.
async fn validate_scan_plan_value<R>(
    repository: &R,
    period_resolution: &Value,
    scan_plan: &Value,
) -> Result<(), String>
where
    R: HoroscopeBuilderCatalog,
{
    let start = period_resolution["start_datetime_utc"]
        .as_str()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    let end = period_resolution["end_datetime_utc"]
        .as_str()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    let start = chrono::DateTime::parse_from_rfc3339(start)
        .map_err(|_| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    let end = chrono::DateTime::parse_from_rfc3339(end)
        .map_err(|_| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    require_canonical_utc_offset(
        period_resolution["start_datetime_utc"]
            .as_str()
            .unwrap_or(""),
        "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
    )?;
    require_canonical_utc_offset(
        period_resolution["end_datetime_utc"].as_str().unwrap_or(""),
        "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH",
    )?;
    let included = period_resolution["included_dates"]
        .as_array()
        .ok_or_else(|| "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH".to_string())?;
    let snapshots = scan_plan["snapshots"]
        .as_array()
        .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
    if scan_plan["snapshot_count"].as_u64() != Some(snapshots.len() as u64) {
        return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
    }
    let scan_profile_code = scan_plan["scan_profile_code"]
        .as_str()
        .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
    let scan_profile = scan_profile(repository, scan_profile_code).await?;
    if snapshots.len() != included.len() * scan_profile.expected_snapshots_per_day {
        return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
    }
    let mut keys = HashSet::new();
    let mut dates = HashSet::new();
    for snapshot in snapshots {
        let key = snapshot["snapshot_key"]
            .as_str()
            .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
        if !keys.insert(key.to_string()) {
            return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
        }
        let date = snapshot["date"]
            .as_str()
            .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
        dates.insert(date.to_string());
        let utc = snapshot["reference_datetime_utc"]
            .as_str()
            .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
        require_canonical_utc_offset(utc, "HOROSCOPE_PERIOD_DATE_RANGE_MISMATCH")?;
        let utc = chrono::DateTime::parse_from_rfc3339(utc)
            .map_err(|_| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
        if utc < start || utc >= end {
            return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
        }
    }
    for date in included.iter().filter_map(Value::as_str) {
        if !dates.contains(date) {
            return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
        }
    }
    Ok(())
}

/// Fonction scan_profile.
async fn scan_profile<R>(repository: &R, scan_profile_code: &str) -> Result<ScanProfile, String>
where
    R: HoroscopeBuilderCatalog,
{
    let row = repository
        .horoscope_scan_profiles()
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .find(|row| row.scan_profile_code == scan_profile_code && row.is_enabled)
        .ok_or_else(|| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())?;
    Ok(ScanProfile {
        granularity: row.granularity,
        reference_time_local: row.reference_time_local,
        expected_snapshots_per_day: row.expected_snapshots_per_day as usize,
    })
}

fn map_period_profile_definition(
    row: HoroscopePeriodProfile,
) -> Result<astral_time_window::PeriodProfileDefinition, String> {
    Ok(astral_time_window::PeriodProfileDefinition {
        period_profile_code: row.period_profile_code,
        resolution_strategy: row.resolution_strategy,
        duration_days: row.duration_days.map(i64::from),
        week_offset: row.week_offset.map(i64::from),
        included_days: row.included_days.unwrap_or_default(),
        is_enabled: row.is_enabled,
    })
}

impl ScanProfile {
    /// Fonction reference_times.
    fn reference_times(&self) -> Result<Vec<NaiveTime>, String> {
        let times = self
            .reference_time_local
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| {
                NaiveTime::parse_from_str(value, "%H:%M")
                    .map_err(|_| "HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if times.len() != self.expected_snapshots_per_day || times.is_empty() {
            return Err("HOROSCOPE_PERIOD_SCAN_PLAN_INVALID".to_string());
        }
        Ok(times)
    }
}
