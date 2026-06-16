use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

pub(crate) fn normalize_rfc3339_utc(raw: &str) -> Result<String, String> {
    DateTime::parse_from_rfc3339(raw)
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
        .map_err(|err| format!("invalid RFC3339 UTC field: {err}"))
}

pub(crate) fn parse_rfc3339(raw: &str) -> Result<DateTime<FixedOffset>, String> {
    DateTime::parse_from_rfc3339(raw).map_err(|err| format!("invalid RFC3339 field: {err}"))
}

pub(crate) fn require_canonical_utc_offset(raw: &str, error_code: &str) -> Result<(), String> {
    let parsed = DateTime::parse_from_rfc3339(raw).map_err(|_| error_code.to_string())?;
    if parsed.with_timezone(&Utc).to_rfc3339() != raw {
        return Err(error_code.to_string());
    }
    Ok(())
}

pub(crate) fn local_to_utc(
    tz: Tz,
    local: NaiveDateTime,
    error_code: &str,
) -> Result<String, String> {
    tz.from_local_datetime(&local)
        .single()
        .ok_or_else(|| error_code.to_string())
        .map(|value| value.with_timezone(&Utc).to_rfc3339())
}

pub(crate) fn reference_datetime_utc(date: &str, timezone: &str, time: &str) -> Option<String> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(time, "%H:%M").ok()?;
    let tz = timezone.parse::<Tz>().ok()?;
    let local = date.and_time(time);
    let resolved = tz.from_local_datetime(&local).single()?;
    Some(resolved.with_timezone(&Utc).to_rfc3339())
}
