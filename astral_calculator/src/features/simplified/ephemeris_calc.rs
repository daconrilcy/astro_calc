#[cfg(feature = "swisseph-engine")]
use std::path::Path;
#[cfg(feature = "swisseph-engine")]
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};

use crate::domain::SignReference;
use crate::shared::astro_math::normalize_degrees;
use crate::shared::error::RuntimeError;

#[cfg(feature = "swisseph-engine")]
pub fn julian_day_utc(datetime: DateTime<Utc>) -> f64 {
    use chrono::{Datelike, Timelike};
    use swiss_eph::safe::julday;

    let hour = datetime.hour() as f64
        + datetime.minute() as f64 / 60.0
        + datetime.second() as f64 / 3600.0
        + f64::from(datetime.nanosecond()) / 3_600_000_000_000.0;

    julday(
        datetime.year(),
        datetime.month() as i32,
        datetime.day() as i32,
        hour,
    )
}

#[cfg(feature = "swisseph-engine")]
pub fn sign_code_at_jd(
    ephemeris_path: &Path,
    jd_ut: f64,
    swe_id: i32,
    signs: &[SignReference],
) -> Result<(String, f64), RuntimeError> {
    use crate::shared::astro_math::zodiac_slot_for_longitude;
    use swiss_eph::safe::{calc_ut, set_ephe_path, CalcFlags};

    let _guard = swiss_ephemeris_lock()
        .lock()
        .map_err(|_| RuntimeError::Ephemeris("Swiss Ephemeris lock poisoned".into()))?;
    set_ephe_path(
        ephemeris_path
            .to_str()
            .ok_or_else(|| RuntimeError::Ephemeris("invalid ephemeris path".into()))?,
    );

    let position = calc_ut(jd_ut, swe_id, CalcFlags::new().with_swiss_ephemeris().raw())
        .map_err(|error| RuntimeError::Ephemeris(error.to_string()))?;

    let longitude = normalize_degrees(position.longitude);
    let slot = zodiac_slot_for_longitude(longitude);
    let sign = signs
        .iter()
        .find(|sign| sign.id == slot)
        .ok_or_else(|| RuntimeError::Ephemeris(format!("missing sign for slot {slot}")))?;

    Ok((sign.code.clone(), longitude))
}

#[cfg(feature = "swisseph-engine")]
fn swiss_ephemeris_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(not(feature = "swisseph-engine"))]
pub fn sign_code_at_jd(
    _ephemeris_path: &std::path::Path,
    _jd_ut: f64,
    _swe_id: i32,
    _signs: &[SignReference],
) -> Result<(String, f64), RuntimeError> {
    Err(RuntimeError::Ephemeris(
        "swisseph-engine feature disabled".into(),
    ))
}

#[cfg(not(feature = "swisseph-engine"))]
pub fn julian_day_utc(_datetime: DateTime<Utc>) -> f64 {
    0.0
}

pub fn dedupe_preserve_order(values: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            out.push(value.clone());
        }
    }
    out
}

pub fn distance_to_sign_boundary_deg(longitude_deg: f64) -> f64 {
    let normalized = normalize_degrees(longitude_deg);
    let within_sign = normalized % 30.0;
    within_sign.min(30.0 - within_sign)
}
